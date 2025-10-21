//! Enrutador muy simple: mapea rutas fijas a funciones de `handlers`.
//! Devuelve un `Route` que indica a qué pool (basic/cpu/io) y qué handler ejecutar.

use crate::handlers::basic;
use crate::workers::HandlerFn;

/// Router sin estado. Lo hacemos `Copy` y `Default` para usarlo fácil en tests.
#[derive(Clone, Copy, Debug, Default)]
pub struct Router;

impl Router {
    /// Construye un router (idéntico a `Default`).
    pub fn new() -> Self {
        Self
    }

    /// Decide el destino de la ruta: pool + handler.
    pub fn route(&self, path: &str) -> Route {
        match path {
            // Básicos / ligeros → pool "basic"
            "/status"    => Route::Basic(basic::status),
            "/timestamp" => Route::Basic(basic::timestamp),
            "/reverse"   => Route::Basic(basic::reverse),
            "/toupper"   => Route::Basic(basic::toupper),
            "/help"      => Route::Basic(basic::help),

            // CPU-bound demostrativos → pool "cpu"
            "/isprime"   => Route::Cpu(basic::isprime),
            "/fibonacci" => Route::Cpu(basic::fibonacci),

            // IO-bound demostrativo (aquí usamos sleep como placeholder) → pool "io"
            "/sleep"     => Route::Io(basic::sleep),

            // Desconocido
            _ => Route::NotFound,
        }
    }
}

/// Resultado del enrutamiento: indica pool y función handler a ejecutar.
#[derive(Clone, Copy, Debug)]
pub enum Route {
    Basic(HandlerFn),
    Cpu(HandlerFn),
    Io(HandlerFn),
    NotFound,
}
