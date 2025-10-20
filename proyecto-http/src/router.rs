//! Enrutador HTTP → "comandos" conocidos.
//!
//! Maneja el enrutamiento de requests HTTP a sus handlers correspondientes.
//! Ahora implementa handlers reales en lugar de placeholders.

use std::collections::HashMap;
use crate::handlers::basic;

#[derive(Clone, Debug)]
pub struct Router;

impl Router {
    /// Construye el router.
    pub fn new() -> Self { Self }

    /// Maneja una request HTTP y devuelve la respuesta JSON correspondiente.
    /// Devuelve `Some(body_json)` si el path existe, o `None` si no existe.
    pub fn handle_early(&self, path: &str, req_id: &str, query_params: &HashMap<String, String>) -> Option<String> {
        match path {
            "/fibonacci" => {
                Some(basic::handle_fibonacci(query_params))
            },
            "/reverse" => {
                Some(basic::handle_reverse(query_params))
            },
            "/toupper" => {
                Some(basic::handle_toupper(query_params))
            },
            "/isprime" => {
                Some(basic::handle_isprime(query_params))
            },
            "/sleep" => {
                Some(basic::handle_sleep(query_params))
            },
            "/help" => {
                Some(basic::handle_help())
            },
            "/status" => {
                Some(format!(
                    r#"{{"status":"ok","message":"Server is running","request_id":"{}"}}"#,
                    req_id
                ))
            },
            "/timestamp" => {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                Some(format!(
                    r#"{{"timestamp":{},"request_id":"{}"}}"#,
                    timestamp, req_id
                ))
            },
            _ => None
        }
    }
}