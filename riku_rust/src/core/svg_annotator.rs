use std::collections::BTreeMap;
use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::core::models::{ChangeKind, DiffReport, Schematic};

static TEXT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"<text[^>]+transform="translate\(([0-9.\-]+),\s*([0-9.\-]+)\)"[^>]*>\s*([^<]+?)\s*</text>"#,
    )
    .expect("valid text regex")
});

static PATH_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"M([\d.\-]+) ([\d.\-]+)L([\d.\-]+) ([\d.\-]+)").expect("valid path regex"));

const COMPONENT_NAME_COLOR: &str = "#cccccc";
const BBOX_HALF: f64 = 15.0;

#[derive(Clone, Copy, Debug)]
struct Transform {
    mooz: f64,
    offset_x: f64,
    offset_y: f64,
    exact: bool,
}

impl Transform {
    fn to_svg(&self, sch_x: f64, sch_y: f64) -> (f64, f64) {
        (
            sch_x * self.mooz + self.offset_x,
            sch_y * self.mooz + self.offset_y,
        )
    }
}

fn change_style(kind: &ChangeKind, cosmetic: bool) -> (&'static str, &'static str) {
    match (kind, cosmetic) {
        (ChangeKind::Added, _) => ("rgba(0,200,0,0.25)", "rgba(0,200,0,0.8)"),
        (ChangeKind::Removed, _) => ("rgba(200,0,0,0.25)", "rgba(200,0,0,0.8)"),
        (ChangeKind::Modified, true) => ("rgba(120,120,120,0.20)", "rgba(120,120,120,0.85)"),
        (ChangeKind::Modified, false) => ("rgba(255,180,0,0.25)", "rgba(255,180,0,0.8)"),
    }
}

fn component_box(
    transform: Transform,
    component: &crate::core::models::Component,
    svg_position: Option<(f64, f64)>,
) -> (f64, f64, f64) {
    let (cx, cy) = svg_position.unwrap_or_else(|| transform.to_svg(component.x, component.y));
    let half = BBOX_HALF * transform.mooz;
    (cx, cy, half)
}

fn extract_name_positions(svg_content: &str) -> BTreeMap<String, (f64, f64)> {
    let mut positions = BTreeMap::new();
    for caps in TEXT_RE.captures_iter(svg_content) {
        let full_match = caps.get(0).map(|m| m.as_str()).unwrap_or("");
        if !full_match.contains(COMPONENT_NAME_COLOR) {
            continue;
        }
        let x = caps[1].parse::<f64>().unwrap_or(0.0);
        let y = caps[2].parse::<f64>().unwrap_or(0.0);
        let text = caps[3].trim().to_string();
        positions.insert(text, (x, y));
    }
    positions
}

fn extract_wire_endpoints(svg_content: &str) -> Vec<(f64, f64)> {
    let mut pts = Vec::new();
    for caps in PATH_RE.captures_iter(svg_content) {
        let x1 = caps[1].parse::<f64>().unwrap_or(0.0);
        let y1 = caps[2].parse::<f64>().unwrap_or(0.0);
        let x2 = caps[3].parse::<f64>().unwrap_or(0.0);
        let y2 = caps[4].parse::<f64>().unwrap_or(0.0);
        pts.push((x1, y1));
        pts.push((x2, y2));
    }
    pts
}

fn lstsq_free(pairs: &[(f64, f64, f64, f64)]) -> Option<Transform> {
    let n = pairs.len() as f64;
    let sum_sx: f64 = pairs.iter().map(|p| p.0).sum();
    let sum_sy: f64 = pairs.iter().map(|p| p.1).sum();
    let sum_vx: f64 = pairs.iter().map(|p| p.2).sum();
    let sum_vy: f64 = pairs.iter().map(|p| p.3).sum();
    let sum_sx2: f64 = pairs.iter().map(|p| p.0 * p.0).sum();
    let sum_sy2: f64 = pairs.iter().map(|p| p.1 * p.1).sum();
    let sum_sxvx: f64 = pairs.iter().map(|p| p.0 * p.2).sum();
    let sum_syvy: f64 = pairs.iter().map(|p| p.1 * p.3).sum();

    let denom_x = n * sum_sx2 - sum_sx * sum_sx;
    let denom_y = n * sum_sy2 - sum_sy * sum_sy;
    if denom_x.abs() < 1e-9 || denom_y.abs() < 1e-9 {
        return None;
    }

    let mooz_x = (n * sum_sxvx - sum_sx * sum_vx) / denom_x;
    let mooz_y = (n * sum_syvy - sum_sy * sum_vy) / denom_y;
    let mooz = (mooz_x + mooz_y) / 2.0;
    Some(Transform {
        mooz,
        offset_x: (sum_vx - mooz * sum_sx) / n,
        offset_y: (sum_vy - mooz * sum_sy) / n,
        exact: false,
    })
}

fn lstsq_fixed_origins(
    pairs: &[(f64, f64, f64, f64)],
    xorigin: f64,
    yorigin: f64,
    svg_wire_pts: Option<&Vec<(f64, f64)>>,
    sch_wire_pts: Option<&Vec<(f64, f64)>>,
) -> Option<Transform> {
    if let (Some(svg_wire_pts), Some(sch_wire_pts)) = (svg_wire_pts, sch_wire_pts) {
        let mut mooz_approx_x = Vec::new();
        for (sch_x, _sch_y, svg_x, _svg_y) in pairs {
            let denom = sch_x + xorigin;
            if denom.abs() > 1e-6 {
                mooz_approx_x.push(svg_x / denom);
            }
        }
        if !mooz_approx_x.is_empty() {
            let mooz_approx = mooz_approx_x.iter().sum::<f64>() / mooz_approx_x.len() as f64;
                let mut wire_pairs = Vec::new();
                for (sx, sy) in sch_wire_pts.iter().copied() {
                    let px = (sx + xorigin) * mooz_approx;
                    let py = (sy + yorigin) * mooz_approx;
                    if let Some(best) = svg_wire_pts.iter().min_by(|a, b| {
                    let da = (a.0 - px).powi(2) + (a.1 - py).powi(2);
                    let db = (b.0 - px).powi(2) + (b.1 - py).powi(2);
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    let dist = ((best.0 - px).powi(2) + (best.1 - py).powi(2)).sqrt();
                    if dist < 8.0 {
                        wire_pairs.push((sx, sy, best.0, best.1));
                    }
                }
            }
            if wire_pairs.len() >= 4 {
                let mut mooz_wx: Vec<f64> = wire_pairs
                    .iter()
                    .filter_map(|(schx, _schy, svgx, _svgy)| {
                        let denom = schx + xorigin;
                        if denom.abs() > 1e-6 {
                            Some(svgx / denom)
                        } else {
                            None
                        }
                    })
                    .collect();
                let mut mooz_wy: Vec<f64> = wire_pairs
                    .iter()
                    .filter_map(|(_schx, schy, _svgx, svgy)| {
                        let denom = schy + yorigin;
                        if denom.abs() > 1e-6 {
                            Some(svgy / denom)
                        } else {
                            None
                        }
                    })
                    .collect();

                if !mooz_wx.is_empty() && !mooz_wy.is_empty() {
                    for samples in [&mut mooz_wx, &mut mooz_wy] {
                        if samples.len() >= 4 {
                            let mean = samples.iter().sum::<f64>() / samples.len() as f64;
                            let variance = samples.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                                / samples.len() as f64;
                            let std = variance.sqrt();
                            samples.retain(|m| (*m - mean).abs() <= 2.0 * std);
                        }
                    }
                    if !mooz_wx.is_empty() && !mooz_wy.is_empty() {
                        let mooz = (mooz_wx.iter().sum::<f64>() / mooz_wx.len() as f64
                            + mooz_wy.iter().sum::<f64>() / mooz_wy.len() as f64)
                            / 2.0;
                        return Some(Transform {
                            mooz,
                            offset_x: xorigin * mooz,
                            offset_y: yorigin * mooz,
                            exact: true,
                        });
                    }
                }
            }
        }
    }

    let mut mooz_samples = Vec::new();
    for (sch_x, _sch_y, svg_x, _svg_y) in pairs {
        let denom = sch_x + xorigin;
        if denom.abs() > 1e-6 {
            mooz_samples.push(svg_x / denom);
        }
    }
    if mooz_samples.is_empty() {
        return None;
    }
    if mooz_samples.len() >= 4 {
        let mean = mooz_samples.iter().sum::<f64>() / mooz_samples.len() as f64;
        let variance =
            mooz_samples.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / mooz_samples.len() as f64;
        let std = variance.sqrt();
        mooz_samples.retain(|m| (*m - mean).abs() <= 2.0 * std);
    }
    if mooz_samples.is_empty() {
        return None;
    }
    let mooz = mooz_samples.iter().sum::<f64>() / mooz_samples.len() as f64;
    Some(Transform {
        mooz,
        offset_x: xorigin * mooz,
        offset_y: yorigin * mooz,
        exact: true,
    })
}

fn fit_transform(
    svg_positions: &BTreeMap<String, (f64, f64)>,
    schematic: &Schematic,
    svg_path: Option<&Path>,
    svg_content: Option<&str>,
) -> Option<Transform> {
    let pairs: Vec<_> = svg_positions
        .iter()
        .filter_map(|(name, (svg_x, svg_y))| {
            schematic.components.get(name).map(|component| {
                (component.x, component.y, *svg_x, *svg_y)
            })
        })
        .collect();

    if pairs.len() < 2 {
        return None;
    }

    if let Some(svg_path) = svg_path {
        let origins_file = svg_path.parent()?.join("origins.txt");
        if origins_file.exists() {
            if let Ok(content) = std::fs::read_to_string(origins_file) {
                let mut lines = content.lines();
                if let (Some(xline), Some(yline)) = (lines.next(), lines.next()) {
                    if let (Ok(xorigin), Ok(yorigin)) = (xline.trim().parse::<f64>(), yline.trim().parse::<f64>()) {
                        let svg_wire_pts = svg_content.map(extract_wire_endpoints);
                        let sch_wire_pts: Option<Vec<(f64, f64)>> = if schematic.wires.is_empty() {
                            None
                        } else {
                            Some(
                                schematic
                                    .wires
                                    .iter()
                                    .flat_map(|w| [(w.x1, w.y1), (w.x2, w.y2)])
                                    .collect(),
                            )
                        };
                        if let Some(t) = lstsq_fixed_origins(
                            &pairs,
                            xorigin,
                            yorigin,
                            svg_wire_pts.as_ref(),
                            sch_wire_pts.as_ref(),
                        ) {
                            return Some(t);
                        }
                    }
                }
            }
        }
    }

    let t = lstsq_free(&pairs)?;
    if pairs.len() < 4 {
        return Some(t);
    }

    let residuals: Vec<f64> = pairs
        .iter()
        .map(|p| {
            let (x, y) = t.to_svg(p.0, p.1);
            (x - p.2).powi(2) + (y - p.3).powi(2)
        })
        .collect();
    let mean = residuals.iter().sum::<f64>() / residuals.len() as f64;
    let variance = residuals.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / residuals.len() as f64;
    let std = variance.sqrt();
    let inliers: Vec<_> = pairs
        .iter()
        .zip(residuals.iter())
        .filter_map(|(pair, residual)| {
            if *residual <= mean + 2.0 * std {
                Some(*pair)
            } else {
                None
            }
        })
        .collect();
    if inliers.len() >= 2 {
        lstsq_free(&inliers)
    } else {
        Some(t)
    }
}

fn wire_elements(
    wires: &[crate::core::models::Wire],
    net_names: &std::collections::BTreeSet<String>,
    kind: &str,
    transform: Transform,
) -> Vec<String> {
    let stroke = match kind {
        "added" => "rgba(0,200,0,0.9)",
        "removed" => "rgba(200,0,0,0.9)",
        _ => "rgba(0,0,0,0.9)",
    };

    wires
        .iter()
        .filter(|w| net_names.contains(&w.label))
        .map(|w| {
            let (x1, y1) = transform.to_svg(w.x1, w.y1);
            let (x2, y2) = transform.to_svg(w.x2, w.y2);
            format!(
                r#"<line x1="{x1:.2}" y1="{y1:.2}" x2="{x2:.2}" y2="{y2:.2}" stroke="{stroke}" stroke-width="2.5" stroke-linecap="round"/>"#
            )
        })
        .collect()
}

pub fn annotate(
    svg_content: &str,
    sch_b: &Schematic,
    diff_report: &DiffReport,
    sch_a: Option<&Schematic>,
    svg_path: Option<&Path>,
) -> String {
    let svg_positions = extract_name_positions(svg_content);
    let transform = match fit_transform(&svg_positions, sch_b, svg_path, Some(svg_content)) {
        Some(t) => t,
        None => return svg_content.to_string(),
    };

    let mut elements = Vec::new();

    for cd in &diff_report.components {
        let source = if cd.kind == crate::core::models::ChangeKind::Removed {
            sch_a.unwrap_or(sch_b)
        } else {
            sch_b
        };
        if let Some(component) = source.components.get(&cd.name) {
            let (fill, stroke) = change_style(&cd.kind, cd.cosmetic);
            let (cx, cy, half) = component_box(transform, component, svg_positions.get(&cd.name).copied());
            elements.push(format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{fill}" stroke="{stroke}" stroke-width="1.5" rx="3" ry="3"/>"#,
                cx - half,
                cy - half,
                2.0 * half,
                2.0 * half
            ));
        }
    }

    if !diff_report.nets_added.is_empty() {
        let set: std::collections::BTreeSet<_> = diff_report.nets_added.iter().cloned().collect();
        elements.extend(wire_elements(&sch_b.wires, &set, "added", transform));
    }
    if let Some(sch_a) = sch_a {
        if !diff_report.nets_removed.is_empty() {
            let set: std::collections::BTreeSet<_> = diff_report.nets_removed.iter().cloned().collect();
            elements.extend(wire_elements(&sch_a.wires, &set, "removed", transform));
        }
    }

    if elements.is_empty() {
        return svg_content.to_string();
    }

    let annotation_layer = format!(
        "\n<g id=\"riku-diff-annotations\">\n{}\n</g>\n",
        elements.join("\n")
    );
    svg_content.replacen("</svg>", &(annotation_layer + "</svg>"), 1)
}

#[cfg(test)]
mod tests {
    use super::{annotate, change_style};
    use crate::core::models::{ChangeKind, Component, DiffReport, Schematic, Wire};
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn styles_cosmetic_changes_differently() {
        assert_eq!(
            change_style(&ChangeKind::Modified, true),
            ("rgba(120,120,120,0.20)", "rgba(120,120,120,0.85)")
        );
        assert_eq!(
            change_style(&ChangeKind::Modified, false),
            ("rgba(255,180,0,0.25)", "rgba(255,180,0,0.8)")
        );
    }

    #[test]
    fn annotate_injects_layer() {
        let mut components = BTreeMap::new();
        components.insert(
            "R1".to_string(),
            Component {
                name: "R1".to_string(),
                symbol: "res.sym".to_string(),
                params: BTreeMap::new(),
                x: 10.0,
                y: 20.0,
                rotation: 0,
                mirror: 0,
            },
        );
        components.insert(
            "R2".to_string(),
            Component {
                name: "R2".to_string(),
                symbol: "cap.sym".to_string(),
                params: BTreeMap::new(),
                x: 30.0,
                y: 10.0,
                rotation: 0,
                mirror: 0,
            },
        );
        let schematic = Schematic {
            components,
            wires: vec![Wire {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 0.0,
                label: "NET1".to_string(),
            }],
            nets: BTreeSet::from(["NET1".to_string()]),
        };
        let diff = DiffReport {
            components: vec![crate::core::models::ComponentDiff {
                name: "R1".to_string(),
                kind: ChangeKind::Added,
                cosmetic: false,
                before: None,
                after: None,
            }],
            nets_added: vec!["NET1".to_string()],
            nets_removed: vec![],
            is_move_all: false,
        };
        let svg = r##"<svg>
<text transform="translate(10, 10)" fill="#cccccc">R1</text>
<text transform="translate(30, 20)" fill="#cccccc">R2</text>
<path d="M0 0L10 0"/>
</svg>"##;

        let out = annotate(svg, &schematic, &diff, None, None);
        assert!(out.contains("riku-diff-annotations"));
        assert!(out.contains("<rect"));
        assert!(out.contains("<line"));
    }
}
