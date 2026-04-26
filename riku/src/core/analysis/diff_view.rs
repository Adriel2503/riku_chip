use std::path::Path;

use thiserror::Error;

use crate::core::analysis::blob_io;
use crate::core::domain::driver::{is_layout_element, is_net_element, net_name, RikuDriver};
use crate::core::domain::git_types::GitError;
use crate::core::domain::models::{ChangeKind, ComponentDiff, DiffReport, Schematic};
use crate::core::domain::ports::GitRepository;
use crate::core::git::git_service::GitService;

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum DiffViewError {
    #[error(transparent)]
    Git(#[from] GitError),
    #[error("no se pudo renderizar: {0}")]
    Render(String),
}

// ─── DiffView ────────────────────────────────────────────────────────────────

/// Vista de diff entre dos commits para un archivo de schematic.
///
/// Contiene todo lo necesario para que cualquier backend (CLI HTML, GUI egui)
/// presente el diff visualmente sin necesidad de re-parsear ni re-renderizar.
pub struct DiffView {
    /// SVG del estado anterior (commit_a), o None si el archivo es nuevo.
    pub svg_a: Option<String>,
    /// SVG del estado posterior (commit_b).
    pub svg_b: String,
    /// Schematic parseado del estado anterior.
    pub sch_a: Option<Schematic>,
    /// Schematic parseado del estado posterior.
    pub sch_b: Schematic,
    /// Reporte de diferencias semánticas.
    pub report: DiffReport,
    /// Advertencias generadas durante el análisis.
    pub warnings: Vec<String>,
}

impl DiffView {
    /// Construye un `DiffView` leyendo blobs de Git y delegando render y diff al driver.
    ///
    /// `commit_a` es el estado anterior, `commit_b` el posterior.
    /// Si el archivo no existe en `commit_a` (archivo nuevo), `svg_a` y `sch_a` son `None`.
    pub fn from_commits(
        repo_path: &Path,
        commit_a: &str,
        commit_b: &str,
        file_path: &str,
        driver: &dyn RikuDriver,
        parse_fn: impl Fn(&[u8]) -> Schematic,
    ) -> Result<Self, DiffViewError> {
        let svc = GitService::open(repo_path)?;
        Self::from_repo(&svc, commit_a, commit_b, file_path, driver, parse_fn)
    }

    /// Versión con repositorio y driver inyectados — facilita testing sin disco.
    pub fn from_repo<R: GitRepository + ?Sized>(
        repo: &R,
        commit_a: &str,
        commit_b: &str,
        file_path: &str,
        driver: &dyn RikuDriver,
        parse_fn: impl Fn(&[u8]) -> Schematic,
    ) -> Result<Self, DiffViewError> {
        let mut warnings = Vec::new();

        // ── Commit B (requerido) ──────────────────────────────────────────
        let content_b = repo
            .get_blob(commit_b, file_path)
            .map_err(DiffViewError::Git)?;
        let sch_b = parse_fn(&content_b);
        let svg_b = driver
            .render(&content_b, file_path)
            .ok_or_else(|| DiffViewError::Render(format!("{file_path} (commit {commit_b})")))?;

        // ── Commit A (opcional — puede no existir si el archivo es nuevo) ─
        let bytes_a = blob_io::read_blob_lenient(repo, commit_a, file_path, &mut warnings)
            .map_err(DiffViewError::Git)?;
        let (svg_a, sch_a, content_a) = match bytes_a {
            Some(bytes) => {
                let sch = parse_fn(&bytes);
                let svg = driver.render(&bytes, file_path);
                (svg, Some(sch), Some(bytes))
            }
            None => (None, None, None),
        };

        // ── Diff semántico ────────────────────────────────────────────────
        let driver_report = driver.diff(content_a.as_deref().unwrap_or(&[]), &content_b, file_path);
        warnings.extend(driver_report.warnings);
        let report = driver_report_to_diff_report(&driver_report.changes);

        Ok(Self {
            svg_a,
            svg_b,
            sch_a,
            sch_b,
            report,
            warnings,
        })
    }
}

// ─── Conversión de tipos ──────────────────────────────────────────────────────

/// Convierte las entradas del driver a `DiffReport` de dominio.
/// Separa componentes, nets y el flag is_move_all en una sola pasada.
pub fn driver_report_to_diff_report(
    changes: &[crate::core::domain::driver::DiffEntry],
) -> DiffReport {
    let mut components = Vec::new();
    let mut nets_added = Vec::new();
    let mut nets_removed = Vec::new();
    let mut is_move_all = false;

    for c in changes {
        if is_layout_element(&c.element) {
            if c.cosmetic {
                is_move_all = true;
            }
            continue;
        }
        if is_net_element(&c.element) {
            match c.kind {
                ChangeKind::Added => nets_added.push(net_name(&c.element).to_string()),
                ChangeKind::Removed => nets_removed.push(net_name(&c.element).to_string()),
                ChangeKind::Modified => {}
            }
            continue;
        }
        components.push(ComponentDiff {
            name: c.element.clone(),
            kind: c.kind.clone(),
            cosmetic: c.cosmetic,
            position_changed: c.position_changed,
            before: c.before.clone(),
            after: c.after.clone(),
        });
    }

    DiffReport {
        components,
        nets_added,
        nets_removed,
        is_move_all,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::domain::driver::{DiffEntry, DriverDiffReport};
    use crate::core::domain::models::FileFormat;

    fn make_report(changes: Vec<DiffEntry>) -> DriverDiffReport {
        DriverDiffReport {
            file_type: FileFormat::Xschem,
            changes,
            ..Default::default()
        }
    }

    #[test]
    fn convierte_componentes_y_nets() {
        let report = make_report(vec![
            DiffEntry {
                kind: ChangeKind::Added,
                element: "R1".to_string(),
                before: None,
                after: Some([("value".to_string(), "10k".to_string())].into()),
                cosmetic: false,
                position_changed: false,
            },
            DiffEntry {
                kind: ChangeKind::Added,
                element: "net:Vdd".to_string(),
                before: None,
                after: None,
                cosmetic: false,
                position_changed: false,
            },
            DiffEntry {
                kind: ChangeKind::Modified,
                element: "layout".to_string(),
                before: None,
                after: None,
                cosmetic: true,
                position_changed: false,
            },
        ]);

        let diff = driver_report_to_diff_report(&report.changes);
        assert_eq!(diff.components.len(), 1);
        assert_eq!(diff.components[0].name, "R1");
        assert_eq!(diff.nets_added, vec!["Vdd"]);
        assert!(diff.is_move_all);
    }

    // ─── Demostración empírica: 4 pasadas vs 1 pasada ────────────────────────

    use std::cell::Cell;

    fn make_n_changes(n: usize) -> Vec<DiffEntry> {
        (0..n)
            .map(|i| DiffEntry {
                kind: ChangeKind::Added,
                element: format!("R{i}"),
                before: None,
                after: None,
                cosmetic: false,
                position_changed: false,
            })
            .collect()
    }

    /// Versión vieja (4 pasadas). Cada `.iter()` recorre el slice completo.
    fn old_version(changes: &[DiffEntry], visits: &Cell<usize>) -> DiffReport {
        let bump = || visits.set(visits.get() + 1);
        DiffReport {
            components: changes
                .iter()
                .inspect(|_| bump())
                .filter(|c| !is_net_element(&c.element) && !is_layout_element(&c.element))
                .map(|c| ComponentDiff {
                    name: c.element.clone(),
                    kind: c.kind.clone(),
                    cosmetic: c.cosmetic,
                    position_changed: c.position_changed,
                    before: c.before.clone(),
                    after: c.after.clone(),
                })
                .collect(),
            nets_added: changes
                .iter()
                .inspect(|_| bump())
                .filter(|c| c.kind == ChangeKind::Added && is_net_element(&c.element))
                .map(|c| net_name(&c.element).to_string())
                .collect(),
            nets_removed: changes
                .iter()
                .inspect(|_| bump())
                .filter(|c| c.kind == ChangeKind::Removed && is_net_element(&c.element))
                .map(|c| net_name(&c.element).to_string())
                .collect(),
            // .any() puede cortar antes — para forzar visita completa usamos
            // .fold con un OR acumulado.
            is_move_all: changes.iter().fold(false, |acc, c| {
                bump();
                acc || (is_layout_element(&c.element) && c.cosmetic)
            }),
        }
    }

    /// Versión nueva (1 pasada).
    fn new_version(changes: &[DiffEntry], visits: &Cell<usize>) -> DiffReport {
        let mut components = Vec::new();
        let mut nets_added = Vec::new();
        let mut nets_removed = Vec::new();
        let mut is_move_all = false;
        for c in changes {
            visits.set(visits.get() + 1);
            if is_layout_element(&c.element) {
                if c.cosmetic {
                    is_move_all = true;
                }
                continue;
            }
            if is_net_element(&c.element) {
                match c.kind {
                    ChangeKind::Added => nets_added.push(net_name(&c.element).to_string()),
                    ChangeKind::Removed => nets_removed.push(net_name(&c.element).to_string()),
                    ChangeKind::Modified => {}
                }
                continue;
            }
            components.push(ComponentDiff {
                name: c.element.clone(),
                kind: c.kind.clone(),
                cosmetic: c.cosmetic,
                position_changed: c.position_changed,
                before: c.before.clone(),
                after: c.after.clone(),
            });
        }
        DiffReport {
            components,
            nets_added,
            nets_removed,
            is_move_all,
        }
    }

    #[test]
    fn old_version_visita_4n_veces() {
        const N: usize = 100;
        let changes = make_n_changes(N);
        let visits = Cell::new(0);
        let _ = old_version(&changes, &visits);
        assert_eq!(visits.get(), 4 * N, "old_version debe visitar 4×N elementos");
        println!("old_version: {} elementos, {} visitas (= 4×N)", N, visits.get());
    }

    #[test]
    fn new_version_visita_n_veces() {
        const N: usize = 100;
        let changes = make_n_changes(N);
        let visits = Cell::new(0);
        let _ = new_version(&changes, &visits);
        assert_eq!(visits.get(), N, "new_version debe visitar exactamente N elementos");
        println!("new_version: {} elementos, {} visitas (= 1×N)", N, visits.get());
    }

    /// Mide wall-clock con N grande. Correr con `cargo test --release -- --nocapture`
    /// para ver la diferencia real (en debug los iteradores no se optimizan).
    #[test]
    #[ignore = "benchmark — correr explícitamente con --release"]
    fn benchmark_old_vs_new() {
        use std::time::Instant;
        const N: usize = 1_000_000;
        const ITERS: usize = 10;
        let changes = make_n_changes(N);
        let visits = Cell::new(0);

        let mut old_total = std::time::Duration::ZERO;
        for _ in 0..ITERS {
            let t = Instant::now();
            let r = old_version(&changes, &visits);
            old_total += t.elapsed();
            std::hint::black_box(r);
            visits.set(0);
        }

        let mut new_total = std::time::Duration::ZERO;
        for _ in 0..ITERS {
            let t = Instant::now();
            let r = new_version(&changes, &visits);
            new_total += t.elapsed();
            std::hint::black_box(r);
            visits.set(0);
        }

        let old_avg = old_total / ITERS as u32;
        let new_avg = new_total / ITERS as u32;
        let ratio = old_avg.as_nanos() as f64 / new_avg.as_nanos().max(1) as f64;
        println!();
        println!("Benchmark con N={N}, {ITERS} iteraciones:");
        println!("  old_version (4 pasadas): {old_avg:?} promedio");
        println!("  new_version (1 pasada):  {new_avg:?} promedio");
        println!("  ratio old/new: {ratio:.2}×");
    }
}
