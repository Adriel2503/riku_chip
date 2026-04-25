//! Formateadores de salida para los comandos.
//!
//! La lógica de dominio nunca conoce el formato. Los comandos eligen el
//! formateador según los flags del CLI.

pub mod log_json;
pub mod log_text;
pub mod status_json;
pub mod status_text;
