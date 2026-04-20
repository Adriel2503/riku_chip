use std::collections::BTreeMap;
use std::iter::Peekable;
use std::str::Lines;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::core::models::{Component, FileFormat, Schematic, Wire};
use crate::core::ports::SchematicParser;

static COMPONENT_HEADER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^C\s+\{(?P<symbol>[^}]*)\}\s+(?P<x>[-+0-9.eE]+)\s+(?P<y>[-+0-9.eE]+)\s+(?P<rot>-?\d+)\s+(?P<mir>-?\d+)\s+\{(?P<body>.*)$",
    )
    .expect("valid xschem component header regex")
});
static WIRE_HEADER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^N\s+(?P<x1>[-+0-9.eE]+)\s+(?P<y1>[-+0-9.eE]+)\s+(?P<x2>[-+0-9.eE]+)\s+(?P<y2>[-+0-9.eE]+)\s+\{(?P<body>.*)$",
    )
    .expect("valid xschem wire header regex")
});

pub fn detect_format(content: &[u8]) -> FileFormat {
    let header = String::from_utf8_lossy(&content[..content.len().min(240)]);
    if header.contains("xschem version=") {
        FileFormat::Xschem
    } else if header.contains("<Qucs Schematic") {
        FileFormat::Qucs
    } else if header.contains("EESchema Schematic File Version") {
        FileFormat::KicadLegacy
    } else {
        FileFormat::Unknown
    }
}

pub fn parse(content: &[u8]) -> Schematic {
    let text = String::from_utf8_lossy(content);
    let mut lines = text.lines().peekable();
    parse_lines(&mut lines)
}

fn parse_lines(lines: &mut Peekable<Lines<'_>>) -> Schematic {
    let mut sch = Schematic::default();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(component) = parse_component_line(trimmed, lines) {
            if let Some(lab) = component.params.get("lab") {
                sch.nets.insert(lab.clone());
            }
            sch.components.insert(component.name.clone(), component);
            continue;
        }

        if let Some(wire) = parse_wire_line(trimmed, lines) {
            if !wire.label.is_empty() {
                sch.nets.insert(wire.label.clone());
            }
            sch.wires.push(wire);
            continue;
        }

        if let Some(label) = parse_net_label_line(trimmed) {
            sch.nets.insert(label);
            continue;
        }
    }

    sch
}

fn parse_component_line(line: &str, lines: &mut Peekable<Lines<'_>>) -> Option<Component> {
    let caps = COMPONENT_HEADER_RE.captures(line)?;
    let symbol = caps.name("symbol")?.as_str().trim().to_string();
    let x = caps.name("x")?.as_str().parse::<f64>().ok()?;
    let y = caps.name("y")?.as_str().parse::<f64>().ok()?;
    let rotation = caps.name("rot")?.as_str().parse::<i32>().ok()?;
    let mirror = caps.name("mir")?.as_str().parse::<i32>().ok()?;
    let body = collect_block_body(caps.name("body")?.as_str(), lines);
    let attrs = parse_attrs(&body);
    let name = attrs.get("name")?.clone();

    let params = attrs
        .into_iter()
        .filter(|(key, _)| key != "name")
        .collect();

    Some(Component {
        name,
        symbol,
        params,
        x,
        y,
        rotation,
        mirror,
    })
}

fn parse_wire_line(line: &str, lines: &mut Peekable<Lines<'_>>) -> Option<Wire> {
    let caps = WIRE_HEADER_RE.captures(line)?;
    let x1 = caps.name("x1")?.as_str().parse::<f64>().ok()?;
    let y1 = caps.name("y1")?.as_str().parse::<f64>().ok()?;
    let x2 = caps.name("x2")?.as_str().parse::<f64>().ok()?;
    let y2 = caps.name("y2")?.as_str().parse::<f64>().ok()?;
    let body = collect_block_body(caps.name("body")?.as_str(), lines);
    let attrs = parse_attrs(&body);
    let label = attrs.get("lab").cloned().unwrap_or_default();

    Some(Wire {
        x1,
        y1,
        x2,
        y2,
        label,
    })
}

fn parse_net_label_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('L') {
        return None;
    }

    let attrs_start = trimmed.find('{')?;
    let attrs = parse_attrs(trimmed[attrs_start + 1..].trim_end_matches('}'));
    attrs.get("lab").cloned()
}

fn collect_block_body(first_line_body: &str, lines: &mut Peekable<Lines<'_>>) -> String {
    let mut body = first_line_body.trim_end_matches('\r').trim().to_string();
    if let Some(stripped) = body.strip_suffix('}') {
        return stripped.trim_end().to_string();
    }

    while let Some(next) = lines.next() {
        let trimmed = next.trim_end_matches('\r');
        let trimmed = trimmed.trim();
        if trimmed == "}" || trimmed == "\"}" {
            break;
        }
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(trimmed);
    }

    body
}

fn parse_attrs(raw: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut chars = raw.chars().peekable();

    while let Some(ch) = skip_separators(&mut chars) {
        if ch == '}' {
            break;
        }

        let mut key = String::new();
        key.push(ch);
        while let Some(&next) = chars.peek() {
            if next == '=' || next.is_whitespace() || next == '}' {
                break;
            }
            key.push(next);
            chars.next();
        }

        skip_whitespace(&mut chars);
        if chars.peek() != Some(&'=') {
            continue;
        }
        chars.next();
        skip_whitespace(&mut chars);

        let value = parse_attr_value(&mut chars);
        out.insert(key, value);
    }

    out
}

fn parse_attr_value(chars: &mut Peekable<std::str::Chars<'_>>) -> String {
    match chars.peek().copied() {
        Some('"') => parse_quoted_value(chars),
        Some('{') => parse_braced_value(chars),
        Some(_) => parse_bare_value(chars),
        None => String::new(),
    }
}

fn parse_quoted_value(chars: &mut Peekable<std::str::Chars<'_>>) -> String {
    let mut value = String::new();
    let mut escaped = false;
    if chars.next() != Some('"') {
        return value;
    }
    while let Some(ch) = chars.next() {
        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => break,
            _ => value.push(ch),
        }
    }
    value
}

fn parse_braced_value(chars: &mut Peekable<std::str::Chars<'_>>) -> String {
    let mut value = String::new();
    let mut depth = 0usize;
    if chars.next() != Some('{') {
        return value;
    }
    depth += 1;
    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                depth += 1;
                value.push(ch);
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                value.push(ch);
            }
            _ => value.push(ch),
        }
    }
    value
}

fn parse_bare_value(chars: &mut Peekable<std::str::Chars<'_>>) -> String {
    let mut value = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            break;
        }
        value.push(ch);
        chars.next();
    }
    value
}

fn skip_separators(chars: &mut Peekable<std::str::Chars<'_>>) -> Option<char> {
    skip_whitespace(chars);
    while let Some(&ch) = chars.peek() {
        if ch == '{' {
            chars.next();
            skip_whitespace(chars);
            continue;
        }
        chars.next();
        return Some(ch);
    }
    None
}

fn skip_whitespace(chars: &mut Peekable<std::str::Chars<'_>>) {
    while matches!(chars.peek(), Some(ch) if ch.is_whitespace()) {
        chars.next();
    }
}

pub struct XschemParser;

impl SchematicParser for XschemParser {
    fn detect_format(&self, content: &[u8]) -> FileFormat {
        detect_format(content)
    }

    fn parse(&self, content: &[u8]) -> Schematic {
        parse(content)
    }
}
