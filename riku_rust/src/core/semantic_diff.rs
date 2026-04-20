use std::collections::{BTreeMap, BTreeSet};

use crate::core::models::{ChangeKind, ComponentDiff, DiffReport};
use crate::parsers::xschem::parse;

fn component_snapshot(
    component: &crate::core::models::Component,
    include_symbol: bool,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    out.insert("x".to_string(), component.x.to_string());
    out.insert("y".to_string(), component.y.to_string());
    out.insert("rotation".to_string(), component.rotation.to_string());
    out.insert("mirror".to_string(), component.mirror.to_string());
    if include_symbol {
        out.insert("symbol".to_string(), component.symbol.clone());
    }
    for (k, v) in &component.params {
        out.insert(k.clone(), v.clone());
    }
    out
}

pub fn diff(content_a: &[u8], content_b: &[u8]) -> DiffReport {
    let sch_a = parse(content_a);
    let sch_b = parse(content_b);
    let mut report = DiffReport::default();

    let names_a: BTreeSet<_> = sch_a.components.keys().cloned().collect();
    let names_b: BTreeSet<_> = sch_b.components.keys().cloned().collect();

    for name in names_a.difference(&names_b) {
        if let Some(component) = sch_a.components.get(name) {
            report.components.push(ComponentDiff {
                name: name.clone(),
                kind: ChangeKind::Removed,
                cosmetic: false,
                before: Some(component_snapshot(component, true)),
                after: None,
            });
        }
    }

    for name in names_b.difference(&names_a) {
        if let Some(component) = sch_b.components.get(name) {
            report.components.push(ComponentDiff {
                name: name.clone(),
                kind: ChangeKind::Added,
                cosmetic: false,
                before: None,
                after: Some(component_snapshot(component, true)),
            });
        }
    }

    let mut coord_only_changes = 0usize;
    let mut coord_only_entries = Vec::new();
    let common: BTreeSet<_> = names_a.intersection(&names_b).cloned().collect();
    for name in &common {
        let ca = &sch_a.components[name];
        let cb = &sch_b.components[name];
        let coords_changed =
            (ca.x, ca.y, ca.rotation, ca.mirror) != (cb.x, cb.y, cb.rotation, cb.mirror);
        let params_changed = ca.params != cb.params || ca.symbol != cb.symbol;

        if params_changed {
            report.components.push(ComponentDiff {
                name: name.clone(),
                kind: ChangeKind::Modified,
                cosmetic: false,
                before: Some(component_snapshot(ca, true)),
                after: Some(component_snapshot(cb, true)),
            });
        } else if coords_changed {
            coord_only_changes += 1;
            coord_only_entries.push(ComponentDiff {
                name: name.clone(),
                kind: ChangeKind::Modified,
                cosmetic: true,
                before: Some(component_snapshot(ca, true)),
                after: Some(component_snapshot(cb, true)),
            });
        }
    }

    report.components.extend(coord_only_entries);

    if !common.is_empty() && coord_only_changes as f64 / common.len() as f64 > 0.8 {
        report.is_move_all = true;
    }

    report.nets_added = sch_b.nets.difference(&sch_a.nets).cloned().collect();
    report.nets_removed = sch_a.nets.difference(&sch_b.nets).cloned().collect();
    report
}
