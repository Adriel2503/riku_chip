//! Constructor `FileSummary::from_report*` y helpers de clasificación.
//!
//! Recorre las entradas de un `DriverDiffReport`, las cuenta, agrega detalles
//! y decide la categoría agregada. La separación del shape (en `types`) y de
//! las etiquetas (en `labels`) deja este archivo enfocado en la lógica de
//! agregación.

use std::collections::BTreeMap;

use crate::core::domain::driver::{
    is_layout_element, is_net_element, net_name, DiffEntry, DriverDiffReport,
};
use crate::core::domain::models::ChangeKind;

use super::labels;
use super::types::{DetailEntry, DetailKind, DetailLevel, FileSummary, SummaryCategory};

impl FileSummary {
    /// Construye un summary desde un `DriverDiffReport` en nivel resumen.
    ///
    /// Conservado por compatibilidad con consumidores de Fase 1. Equivale a
    /// `from_report_with(report, path, DetailLevel::Resumen)`.
    pub fn from_report(report: &DriverDiffReport, path: &str) -> Self {
        Self::from_report_with(report, path, DetailLevel::Resumen)
    }

    /// Construye un summary desde un `DriverDiffReport` con el nivel solicitado.
    pub fn from_report_with(report: &DriverDiffReport, path: &str, level: DetailLevel) -> Self {
        let agg = aggregate_changes(report, level);
        let category = decide_category(agg.semantic, agg.cosmetic);
        let full_report = matches!(level, DetailLevel::Completo).then(|| report.clone());

        Self {
            path: path.to_string(),
            format: report.file_type.clone(),
            category,
            counts: agg.counts,
            details: agg.details,
            full_report,
            errors: Vec::new(),
        }
    }
}

/// Resultado de recorrer las entradas de un `DriverDiffReport`: contadores
/// agregados por tipo de cambio, detalles opcionales y conteos brutos de
/// semánticos/cosméticos para que el caller decida la categoría.
struct Aggregated {
    counts: BTreeMap<String, i64>,
    details: Vec<DetailEntry>,
    semantic: i64,
    cosmetic: i64,
}

fn aggregate_changes(report: &DriverDiffReport, level: DetailLevel) -> Aggregated {
    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    let mut details: Vec<DetailEntry> = Vec::new();
    let mut semantic = 0i64;
    let mut cosmetic = 0i64;

    for change in &report.changes {
        if change.cosmetic {
            cosmetic += 1;
            continue;
        }
        semantic += 1;

        if is_layout_element(&change.element) {
            continue;
        }

        let is_net = is_net_element(&change.element);
        let (count_key, detail_kind) = classify(is_net, &change.kind, &change.element);
        *counts.entry(count_key.to_string()).or_insert(0) += 1;

        if matches!(level, DetailLevel::Detalle | DetailLevel::Completo) {
            details.push(DetailEntry {
                kind: detail_kind,
                element: net_name(&change.element).to_string(),
                params: extract_param_changes(change),
            });
        }
    }

    Aggregated {
        counts,
        details,
        semantic,
        cosmetic,
    }
}

/// Regla de negocio: prioridad `Semantic > Cosmetic > Unchanged`.
fn decide_category(semantic: i64, cosmetic: i64) -> SummaryCategory {
    if semantic > 0 {
        SummaryCategory::Semantic
    } else if cosmetic > 0 {
        SummaryCategory::Cosmetic
    } else {
        SummaryCategory::Unchanged
    }
}

fn classify(is_net: bool, kind: &ChangeKind, element: &str) -> (&'static str, DetailKind) {
    match (is_net, kind) {
        (true, ChangeKind::Added) => (labels::NETS_ADDED, DetailKind::NetAdded),
        (true, ChangeKind::Removed) => (labels::NETS_REMOVED, DetailKind::NetRemoved),
        (true, ChangeKind::Modified) => (labels::NETS_MODIFIED, DetailKind::NetModified),
        (false, ChangeKind::Added) => (labels::COMPONENTS_ADDED, DetailKind::ComponentAdded),
        (false, ChangeKind::Removed) => (labels::COMPONENTS_REMOVED, DetailKind::ComponentRemoved),
        (false, ChangeKind::Modified) => {
            if element.contains(" → ") {
                (labels::COMPONENTS_RENAMED, DetailKind::ComponentRenamed)
            } else {
                (labels::COMPONENTS_MODIFIED, DetailKind::ComponentModified)
            }
        }
    }
}

/// Extrae cambios de parámetros (key: "before → after") ignorando posición y
/// rotación, que son cosméticos y ya filtrados por el driver pero pueden
/// aparecer en el mapa.
fn extract_param_changes(entry: &DiffEntry) -> BTreeMap<String, String> {
    let (before, after) = match (&entry.before, &entry.after) {
        (Some(b), Some(a)) => (b, a),
        _ => return BTreeMap::new(),
    };
    let mut out = BTreeMap::new();
    for key in before.keys().chain(after.keys()) {
        if matches!(key.as_str(), "x" | "y" | "rotation" | "mirror") {
            continue;
        }
        let b = before.get(key);
        let a = after.get(key);
        match (b, a) {
            (Some(bv), Some(av)) if bv != av => {
                out.insert(key.clone(), format!("{bv} → {av}"));
            }
            (None, Some(av)) => {
                out.insert(key.clone(), format!("(nuevo) → {av}"));
            }
            (Some(bv), None) => {
                out.insert(key.clone(), format!("{bv} → (eliminado)"));
            }
            _ => {}
        }
    }
    out
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::models::FileFormat;

    fn entry(kind: ChangeKind, element: &str, cosmetic: bool) -> DiffEntry {
        DiffEntry {
            kind,
            element: element.to_string(),
            before: None,
            after: None,
            cosmetic,
            position_changed: false,
        }
    }

    fn report(entries: Vec<DiffEntry>) -> DriverDiffReport {
        DriverDiffReport {
            file_type: FileFormat::Xschem,
            changes: entries,
            ..Default::default()
        }
    }

    #[test]
    fn solo_cosmeticos_es_categoria_cosmetic() {
        let r = report(vec![entry(ChangeKind::Modified, "layout", true)]);
        let s = FileSummary::from_report(&r, "a.sch");
        assert_eq!(s.category, SummaryCategory::Cosmetic);
        assert!(s.counts.is_empty());
    }

    #[test]
    fn semantico_cuenta_componentes_y_nets() {
        let r = report(vec![
            entry(ChangeKind::Added, "M1", false),
            entry(ChangeKind::Added, "M2", false),
            entry(ChangeKind::Removed, "net:vbias", false),
            entry(ChangeKind::Modified, "vin → vin_diff", false),
        ]);
        let s = FileSummary::from_report(&r, "a.sch");
        assert_eq!(s.category, SummaryCategory::Semantic);
        assert_eq!(s.counts.get(labels::COMPONENTS_ADDED), Some(&2));
        assert_eq!(s.counts.get(labels::NETS_REMOVED), Some(&1));
        assert_eq!(s.counts.get(labels::COMPONENTS_RENAMED), Some(&1));
    }

    #[test]
    fn sin_cambios_es_unchanged() {
        let r = report(vec![]);
        let s = FileSummary::from_report(&r, "a.sch");
        assert_eq!(s.category, SummaryCategory::Unchanged);
    }

    #[test]
    fn cambios_en_layout_no_cuentan_pero_marcan_cosmetic() {
        let r = report(vec![entry(ChangeKind::Modified, "layout", true)]);
        let s = FileSummary::from_report(&r, "a.sch");
        assert_eq!(s.category, SummaryCategory::Cosmetic);
        assert!(s.counts.is_empty());
    }

    #[test]
    fn nivel_resumen_no_llena_details_ni_full_report() {
        let r = report(vec![entry(ChangeKind::Added, "M1", false)]);
        let s = FileSummary::from_report_with(&r, "a.sch", DetailLevel::Resumen);
        assert!(s.details.is_empty());
        assert!(s.full_report.is_none());
    }

    #[test]
    fn nivel_detalle_llena_details_pero_no_full_report() {
        let r = report(vec![entry(ChangeKind::Added, "M1", false)]);
        let s = FileSummary::from_report_with(&r, "a.sch", DetailLevel::Detalle);
        assert_eq!(s.details.len(), 1);
        assert_eq!(s.details[0].kind, DetailKind::ComponentAdded);
        assert_eq!(s.details[0].element, "M1");
        assert!(s.full_report.is_none());
    }

    #[test]
    fn nivel_completo_llena_todo() {
        let r = report(vec![entry(ChangeKind::Added, "M1", false)]);
        let s = FileSummary::from_report_with(&r, "a.sch", DetailLevel::Completo);
        assert_eq!(s.details.len(), 1);
        assert!(s.full_report.is_some());
    }

    #[test]
    fn decide_category_prioriza_semantic_sobre_cosmetic() {
        assert_eq!(decide_category(1, 5), SummaryCategory::Semantic);
        assert_eq!(decide_category(0, 3), SummaryCategory::Cosmetic);
        assert_eq!(decide_category(0, 0), SummaryCategory::Unchanged);
    }

    #[test]
    fn detalle_extrae_cambios_de_parametros() {
        let mut before = BTreeMap::new();
        before.insert("W".to_string(), "4u".to_string());
        before.insert("L".to_string(), "180n".to_string());
        before.insert("x".to_string(), "100".to_string()); // debe ignorarse
        let mut after = BTreeMap::new();
        after.insert("W".to_string(), "8u".to_string());
        after.insert("L".to_string(), "180n".to_string());
        after.insert("x".to_string(), "200".to_string());

        let mut e = entry(ChangeKind::Modified, "M3", false);
        e.before = Some(before);
        e.after = Some(after);
        let r = report(vec![e]);

        let s = FileSummary::from_report_with(&r, "a.sch", DetailLevel::Detalle);
        let d = &s.details[0];
        assert_eq!(d.params.get("W").map(String::as_str), Some("4u → 8u"));
        assert!(!d.params.contains_key("x"));
        assert!(!d.params.contains_key("L"));
    }
}
