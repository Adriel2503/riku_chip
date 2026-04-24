//! Adaptador fino sobre `xschem_viewer::semantic::parse_semantic`.
//!
//! Riku necesita un `Schematic` (alias del `SemanticSchematic` de la librería)
//! para sus drivers y reportes. La construcción de la escena + resolución de
//! conectividad vive en xschem-viewer; este módulo solo añade la detección
//! de formato y configura las opciones de render.

use crate::core::models::{FileFormat, Schematic};
use crate::core::ports::SchematicParser;

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
    let text = match std::str::from_utf8(content) {
        Ok(s) => s,
        Err(_) => return Schematic::default(),
    };
    let opts = xschem_viewer::RenderOptions::dark().with_sym_paths_from_xschemrc();
    xschem_viewer::semantic::parse_semantic(text, &opts)
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
