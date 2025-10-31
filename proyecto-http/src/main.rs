/*Punto de entrada del binario.

Crea la configuración (Config), el router (Router) y el estado compartido (Shared = Arc<AppState>).

Llama al bucle del listener HTTP (http_listen_loop) que se queda aceptando conexiones.

En Sprint 1, acá agregaremos lectura de CLI/env y (posible) inicialización de pools por comando y JobManager. */

mod core;      // Núcleo HTTP: AppState/Shared, Request/Response, listener.
mod router;    // Tabla de rutas y enum Command.
mod workers;   // Interfaces de colas/handlers (esqueleto en Sprint 0).
mod jobs;      // Contratos del Job Manager (stub en Sprint 0).
mod metrics;   // Estructuras básicas de métricas.
mod config;    // Carga de configuración (por ahora, defaults).
mod handlers;  // Namespaces para endpoints (vacíos en Sprint 0).

use crate::config::Config;
use crate::core::{http_listen_loop, AppState, Shared};
use crate::router::Router;
use crate::jobs::spawn_dispatcher;

fn main() {
    // 1) Cargar configuración
    //
    // Por ahora usamos valores por defecto (puerto 8080).
    // En Sprint 1 agregaremos lectura desde variables de entorno y/o CLI:
    //   PROY_PORT=9090 cargo run
    //   o flags estilo --port 9090 (con un pequeño parser).
    let cfg = Config::from_env_or_default();
    println!("[CONFIG] {:?}", cfg); 

    // 2) Construir el router
    //
    // El Router conoce todas las rutas que soportará el servidor.
    // En este Sprint, `handle_early()` retorna un JSON "placeholder"
    // si la ruta existe (200) o 404 si no existe.
    let router = Router::new();

    // 3) Estado compartido (AppState) dentro de un Arc
    //
    // `Shared` es un alias a Arc<AppState>: esto nos permitirá, en
    // sprints posteriores, compartir config/métricas/router/pools con
    // los hilos que procesarán conexiones y jobs.
    let shared: Shared = AppState::shared(cfg.clone(), router);
    spawn_dispatcher(shared.clone());


    // 4) Arrancar el loop del listener HTTP/1.0
    //
    // - Se hace bind() al puerto (0.0.0.0:PUERTO).
    // - Acepta conexiones con accept().
    // - Lee el request (máx 8KB en Sprint 0), parsea método/ruta/headers.
    // - Construye un `Request` con id (X-Request-Id) y datos parseados.
    // - Llama al router; si no hay handler real, responde placeholder.
    //
    // Si el loop devuelve Err (p. ej., puerto ocupado o error fatal),
    // lo reportamos y termina el proceso con código distinto de 0.
    if let Err(e) = http_listen_loop(&shared) {
        eprintln!("[FATAL] listener terminó con error: {e}");
        std::process::exit(1);
    }
}