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
            // Endpoints básicos implementados
            "/fibonacci" => Some(basic::handle_fibonacci(query_params)),
            "/reverse" => Some(basic::handle_reverse(query_params)),
            "/toupper" => Some(basic::handle_toupper(query_params)),
            "/isprime" => Some(basic::handle_isprime(query_params)),
            "/sleep" => Some(basic::handle_sleep(query_params)),
            "/random" => Some(basic::handle_random(query_params)),
            "/hash" => Some(basic::handle_hash(query_params)),
            "/simulate" => Some(basic::handle_simulate(query_params)),
            "/loadtest" => Some(basic::handle_loadtest(query_params)),
            "/createfile" => Some(basic::handle_createfile(query_params)),
            "/deletefile" => Some(basic::handle_deletefile(query_params)),
            "/help" => Some(basic::handle_help()),
            
            // Endpoints del sistema
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
            
            // Endpoints CPU-bound (por implementar)
            "/factor" | "/pi" | "/mandelbrot" | "/matrixmul" => {
                Some(format!(
                    r#"{{"status":"not_implemented","path":"{}","message":"CPU-bound handler not yet implemented","request_id":"{}"}}"#,
                    path, req_id
                ))
            },
            
            // Endpoints IO-bound (por implementar)
            "/sortfile" | "/wordcount" | "/grep" | "/compress" | "/hashfile" => {
                Some(format!(
                    r#"{{"status":"not_implemented","path":"{}","message":"IO-bound handler not yet implemented","request_id":"{}"}}"#,
                    path, req_id
                ))
            },
            
            // Sistema de Jobs (por implementar)
            "/jobs/submit" | "/jobs/status" | "/jobs/result" | "/jobs/cancel" => {
                Some(format!(
                    r#"{{"status":"not_implemented","path":"{}","message":"Job system not yet implemented","request_id":"{}"}}"#,
                    path, req_id
                ))
            },
            
            // Métricas (por implementar)
            "/metrics" => {
                Some(format!(
                    r#"{{"status":"not_implemented","path":"{}","message":"Metrics endpoint not yet implemented","request_id":"{}"}}"#,
                    path, req_id
                ))
            },
            
            _ => None
        }
    }
}