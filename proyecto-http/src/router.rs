//! Enrutador muy simple: mapea rutas fijas a funciones de `handlers`.

use crate::core::{Request, Shared};
use crate::handlers;

#[derive(Clone)]
pub struct Router;

impl Router {
    pub fn new() -> Self { Self }

    /// Decide qué handler ejecutar según `req.path`.
    /// Devuelve `(status_code, content_type, body_bytes)`.
    pub fn route(&self, state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
        match req.path.as_str() {
            "/status"    => handlers::status(state, req),
            "/timestamp" => handlers::timestamp(state, req),
            "/reverse"   => handlers::reverse(state, req),
            "/toupper"   => handlers::toupper(state, req),
            _ => handlers::not_found(state, req),
        }
    }
}
