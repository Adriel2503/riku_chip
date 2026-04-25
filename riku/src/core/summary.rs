//! Vistas agregadas (`Summary`) sobre un `DriverDiffReport`.
//!
//! `riku status` y `riku log` necesitan presentar muchos archivos en una sola
//! pantalla. El `DriverDiffReport` completo es demasiado verboso para eso —
//! `FileSummary` es una agregación pensada para listas: pocas claves, fácil de
//! formatear en una línea, y categorizada (semantic / cosmetic / unchanged).
//!
//! El mapa `counts` es flexible a propósito: cada driver decide qué eventos
//! reporta (`components_added`, `nets_renamed`, `polygons_added_M1`, ...). El
//! formateador traduce las claves conocidas a etiquetas humanas; las que no
//! conoce las muestra tal cual. Eso permite añadir formatos sin tocar el core.
//!
//! Las claves canónicas aceptadas por el formateador de texto se documentan
//! en [`labels`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::core::driver::DriverDiffReport;
use crate::core::models::{ChangeKind, FileFormat};

/// Categoría agregada de un archivo en una lista (`riku status`, `riku log`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryCategory {
    /// Hay al menos un cambio no-cosmético.
    Semantic,
    /// Hubo cambios pero todos cosméticos (reposicionamiento, etc.).
    Cosmetic,
    /// Driver no detectó ningún cambio.
    Unchanged,
    /// No hay driver para este formato — Riku no opina.
    Unknown,
    /// El driver crasheó o el blob no se pudo leer.
    Error,
}

impl SummaryCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Semantic => "semantic",
            Self::Cosmetic => "cosmetic",
            Self::Unchanged => "unchanged",
            Self::Unknown => "unknown",
            Self::Error => "error",
        }
    }
}

/// Vista resumida de un archivo, lista para ser mostrada en una línea.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileSummary {
    pub path: String,
    pub format: FileFormat,
    pub category: SummaryCategory,
    /// Eventos agregados — claves canónicas en [`labels`], otras pasan tal cual.
    pub counts: BTreeMap<String, i64>,
    /// Mensajes de error si `category == Error`. Vacío en otros casos.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<String>,
}

impl FileSummary {
    /// Construye un summary desde un `DriverDiffReport`.
    pub fn from_report(report: &DriverDiffReport, path: &str) -> Self {
        let mut counts: BTreeMap<String, i64> = BTreeMap::new();
        let mut semantic_changes = 0i64;
        let mut cosmetic_changes = 0i64;

        for change in &report.changes {
            if change.cosmetic {
                cosmetic_changes += 1;
                continue;
            }
            semantic_changes += 1;

            let is_net = change.element.starts_with("net:");
            let is_layout = change.element == "layout";
            if is_layout {
                continue;
            }

            let key = match (is_net, &change.kind) {
                (true, ChangeKind::Added) => labels::NETS_ADDED,
                (true, ChangeKind::Removed) => labels::NETS_REMOVED,
                (true, ChangeKind::Modified) => labels::NETS_MODIFIED,
                (false, ChangeKind::Added) => labels::COMPONENTS_ADDED,
                (false, ChangeKind::Removed) => labels::COMPONENTS_REMOVED,
                (false, ChangeKind::Modified) => {
                    if change.element.contains(" → ") {
                        labels::COMPONENTS_RENAMED
                    } else {
                        labels::COMPONENTS_MODIFIED
                    }
                }
            };
            *counts.entry(key.to_string()).or_insert(0) += 1;
        }

        let category = if semantic_changes > 0 {
            SummaryCategory::Semantic
        } else if cosmetic_changes > 0 {
            SummaryCategory::Cosmetic
        } else {
            SummaryCategory::Unchanged
        };

        Self {
            path: path.to_string(),
            format: report.file_type.clone(),
            category,
            counts,
            errors: Vec::new(),
        }
    }

    pub fn unknown(path: &str) -> Self {
        Self {
            path: path.to_string(),
            format: FileFormat::Unknown,
            category: SummaryCategory::Unknown,
            counts: BTreeMap::new(),
            errors: Vec::new(),
        }
    }

    pub fn error(path: &str, message: impl Into<String>) -> Self {
        Self {
            path: path.to_string(),
            format: FileFormat::Unknown,
            category: SummaryCategory::Error,
            counts: BTreeMap::new(),
            errors: vec![message.into()],
        }
    }
}

/// Claves canónicas para el mapa `counts`. Mantenidas como constantes para
/// evitar typos y facilitar refactor.
pub mod labels {
    pub const COMPONENTS_ADDED: &str = "components_added";
    pub const COMPONENTS_REMOVED: &str = "components_removed";
    pub const COMPONENTS_MODIFIED: &str = "components_modified";
    pub const COMPONENTS_RENAMED: &str = "components_renamed";
    pub const NETS_ADDED: &str = "nets_added";
    pub const NETS_REMOVED: &str = "nets_removed";
    pub const NETS_MODIFIED: &str = "nets_modified";
}

/// Traduce una clave canónica a etiqueta corta humana (singular/plural).
/// Devuelve `None` si la clave no es canónica — el formateador puede entonces
/// imprimir la clave tal cual.
pub fn label_for(key: &str, count: i64) -> Option<String> {
    let plural = count.abs() != 1;
    let label = match key {
        labels::COMPONENTS_ADDED if !plural => "componente añadido",
        labels::COMPONENTS_ADDED => "componentes añadidos",
        labels::COMPONENTS_REMOVED if !plural => "componente eliminado",
        labels::COMPONENTS_REMOVED => "componentes eliminados",
        labels::COMPONENTS_MODIFIED if !plural => "componente modificado",
        labels::COMPONENTS_MODIFIED => "componentes modificados",
        labels::COMPONENTS_RENAMED if !plural => "componente renombrado",
        labels::COMPONENTS_RENAMED => "componentes renombrados",
        labels::NETS_ADDED if !plural => "net añadida",
        labels::NETS_ADDED => "nets añadidas",
        labels::NETS_REMOVED if !plural => "net eliminada",
        labels::NETS_REMOVED => "nets eliminadas",
        labels::NETS_MODIFIED if !plural => "net modificada",
        labels::NETS_MODIFIED => "nets modificadas",
        _ => return None,
    };
    Some(label.to_string())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::driver::DiffEntry;

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
    fn label_for_singular_y_plural() {
        assert_eq!(label_for(labels::COMPONENTS_ADDED, 1).unwrap(), "componente añadido");
        assert_eq!(label_for(labels::COMPONENTS_ADDED, 3).unwrap(), "componentes añadidos");
        assert_eq!(label_for("clave_desconocida", 1), None);
    }

    #[test]
    fn cambios_en_layout_no_cuentan_pero_marcan_cosmetic() {
        let r = report(vec![entry(ChangeKind::Modified, "layout", true)]);
        let s = FileSummary::from_report(&r, "a.sch");
        assert_eq!(s.category, SummaryCategory::Cosmetic);
        assert!(s.counts.is_empty());
    }
}
