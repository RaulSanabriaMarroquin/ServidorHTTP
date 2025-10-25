//! Enrutador completo: mapea todas las rutas a funciones de `handlers`.
//! Devuelve un `Route` que indica a qué pool (basic/cpu/io) y qué handler ejecutar.

use crate::handlers::{basic, cpu, io, jobs};
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
            "/status"     => Route::Basic(basic::status),
            "/timestamp"  => Route::Basic(basic::timestamp),
            "/reverse"    => Route::Basic(basic::reverse),
            "/toupper"    => Route::Basic(basic::toupper),
            "/help"       => Route::Basic(basic::help),
            "/random"     => Route::Basic(basic::random),
            "/hash"       => Route::Basic(basic::hash),
            //"/simulate"   => Route::Basic(basic::simulate),
            //"/loadtest"   => Route::Basic(basic::loadtest),
            "/createfile" => Route::Basic(basic::createfile),
            "/deletefile" => Route::Basic(basic::deletefile),
            "/fibonacci"  => Route::Cpu(basic::fibonacci),

            // CPU-bound → pool "cpu"
            "/isprime"    => Route::Cpu(cpu::isprime),
            "/factor"     => Route::Cpu(cpu::factor),
            "/pi"         => Route::Cpu(cpu::pi),
            "/mandelbrot" => Route::Cpu(cpu::mandelbrot),
            "/matrixmul"  => Route::Cpu(cpu::matrixmul),

            // IO-bound → pool "io"
            "/sleep"      => Route::Io(basic::sleep),
            "/sortfile"   => Route::Io(io::sortfile),
            "/wordcount"  => Route::Io(io::wordcount),
            "/grep"       => Route::Io(io::grep),
            "/compress"   => Route::Io(io::compress),
            "/hashfile"   => Route::Io(io::hashfile),

            // Sistema de Jobs
            "/jobs/submit" => Route::Basic(jobs::submit),
            "/jobs/status" => Route::Basic(jobs::status),
            "/jobs/result" => Route::Basic(jobs::result),
            "/jobs/cancel" => Route::Basic(jobs::cancel),

            // Métricas
            "/metrics" => Route::Basic(basic::metrics),

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
