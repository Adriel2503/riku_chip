//! Formateadores de salida para los comandos.
//!
//! Cada submódulo entrega una representación distinta del mismo
//! `StatusReport` (o futuros `LogReport`). Los comandos eligen el formateador
//! según los flags del CLI; la lógica de dominio nunca conoce el formato.

pub mod json;
pub mod text;
