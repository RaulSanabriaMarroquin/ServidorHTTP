//! Núcleo HTTP/1.0 (Sprint 0/1/2 - con pools de workers).
//!
//! Expone:
//! - `AppState`  : estado global (config, router, timestamps, métricas, pools).
//! - `Shared`    : alias `Arc<AppState>` para compartir estado entre hilos.
//! - `Request`   : representación mínima de una petición HTTP (GET).
//! - `http_listen_loop()` : loop principal que acepta conexiones.
//! - `handle_connection()` : lee → parsea → rutea → encola tarea en pool → worker responde.
//!
//! Notas clave de diseño:
//! - HTTP/1.0 sin keep-alive: una petición por conexión y se cierra.
//! - Solo GET en Sprint 0/1. Otros métodos devuelven 501.
//! - La query `?a=1&b=2` se parsea a `HashMap<String, String>`.
//! - Métricas mínimas (`accepted`/`handled`) protegidas con Mutex (requisito del curso).
//! - En Sprint 2, la ejecución del handler se hace en un pool (basic/cpu/io) y
//!   el worker escribe la respuesta en el `TcpStream` clonado.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use crate::config::Config;
use crate::metrics::Metrics; // Arc<Mutex<...>> por dentro
use crate::router::{Route, Router};
use crate::workers::{Backpressure, HandlerFn, Pools, WorkQueue};
use crate::jobs::JobStore;

/// Representa una petición HTTP simplificada para nuestros handlers.
#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
    pub http_version: String,
    pub request_id: String,
}

/// Estado global del servidor (se comparte entre hilos con Arc).
#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub router: Router,
    pub started_ms: u128,
    pub metrics: Arc<Metrics>, // contadores protegidos con Mutex
    pub pools: Arc<Pools>,     // pools de workers (basic/cpu/io)
    pub job_store: Arc<JobStore>, // sistema de jobs
}

/// Alias práctico: `Shared` es un `Arc<AppState>`.
pub type Shared = Arc<AppState>;

/// Marca de tiempo UNIX en milisegundos (u128).
pub fn now_ms_since_epoch() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

impl AppState {
    /// Construcción en DOS PASOS porque `Pools::new(&Shared)` necesita el Shared ya creado:
    /// 1) Creamos un `AppState` dummy con `Pools::new_dummy()`.
    /// 2) Construimos los Pools reales con `Pools::new(&dummy)`.
    /// 3) Retornamos un `AppState` igual al dummy pero con `pools` reales.
    pub fn shared(cfg: Config, router: Router) -> Shared {
        let started_ms = now_ms_since_epoch();

        // Paso 1: AppState "dummy"
        let dummy = Arc::new(AppState {
            cfg,
            router,
            started_ms,
            metrics: Arc::new(Metrics::default()),
            pools: Arc::new(Pools::new_dummy()),
            job_store: Arc::new(JobStore::new()),
        });

        // Paso 2: Pools reales, ahora que tenemos &Shared disponible
        let pools_real = Arc::new(Pools::new(&dummy));

        // Paso 3: construir el AppState definitivo sustituyendo los pools
        Arc::new(AppState {
            cfg: dummy.cfg.clone(),
            router: dummy.router.clone(),
            started_ms: dummy.started_ms,
            metrics: Arc::clone(&dummy.metrics),
            pools: pools_real,
            job_store: Arc::clone(&dummy.job_store),
        })
    }
}

/// Inicia el listener HTTP/1.0 y atiende conexiones en hilos cortos.
pub fn http_listen_loop(state: &Shared) -> std::io::Result<()> {
    let addr = format!("0.0.0.0:{}", state.cfg.port);
    let listener = TcpListener::bind(&addr)?;
    println!("[INFO] listening on http://{} (HTTP/1.0)", addr);

    let mut next_id: u64 = 1;

    for stream in listener.incoming() {
        match stream {
            Ok(mut s) => {
                // ++accepted con Mutex (requisito Arc/Mutex)
                state.metrics.inc_accepted();

                let req_id = format!("req-{}", next_id);
                next_id += 1;

                let state_cloned = Arc::clone(state);
                thread::spawn(move || {
                    if let Err(e) = handle_connection(&state_cloned, &mut s, &req_id) {
                        eprintln!("[WARN] connection error ({}): {}", req_id, e);
                    }
                    // ++handled con Mutex
                    state_cloned.metrics.inc_handled();
                });
            }
            Err(e) => eprintln!("[WARN] accept error: {e}"),
        }
    }
    Ok(())
}

/// Lee UNA petición (HTTP/1.0), la parsea y la encola en el pool según la ruta.
/// El worker ejecuta el handler y escribe la respuesta en el stream clonado.
pub fn handle_connection(
    state: &Shared,
    stream: &mut TcpStream,
    req_id: &str,
) -> std::io::Result<()> {
    // 1) leer request
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let raw = String::from_utf8_lossy(&buf[..n]);

    // 2) parsear request-line
    let mut lines = raw.lines();
    let req_line = lines.next().unwrap_or_default();
    let mut parts = req_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    let _httpver = parts.next().unwrap_or("HTTP/1.0");

    // 3) Solo GET por ahora
    if method != "GET" {
        let body = format!(
            r#"{{"error":"not_implemented","method":"{}","request_id":"{}"}}"#,
            method, req_id
        );
        return write_response(stream, 501, "Not Implemented", req_id, body.as_bytes());
    }

    // 4) path + query
    let (path, query_map) = if let Some((p, q)) = target.split_once('?') {
        (p.to_string(), parse_query(q))
    } else {
        (target.to_string(), HashMap::new())
    };

    // 5) construir Request para el handler
    let req = Request {
        method: "GET".into(),
        path: path.clone(),
        query: query_map,
        http_version: "HTTP/1.0".into(),
        request_id: req_id.to_string(),
    };

    // 6) resolver ruta → (pool, handler)
    match state.router.route(&req.path) {
        Route::Basic(handler) => enqueue_on_pool(stream, req, handler, &state.pools.basic, state),
        Route::Cpu(handler) => enqueue_on_pool(stream, req, handler, &state.pools.cpu, state),
        Route::Io(handler) => enqueue_on_pool(stream, req, handler, &state.pools.io, state),
        Route::NotFound => {
            let body = format!(
                r#"{{"error":"not_found","path":"{}","request_id":"{}"}}"#,
                path, req_id
            );
            write_response(stream, 404, "Not Found", req_id, body.as_bytes())
        }
    }
}

/// Convierte la query-string `a=1&b=2` en `HashMap`.
fn parse_query(qs: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for pair in qs.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            m.insert(k.to_string(), v.to_string());
        }
    }
    m
}

/// Extrae el nombre del comando desde el path.
/// Ejemplos: "/isprime?n=97" -> "isprime", "/jobs/submit" -> "jobs/submit"
fn extract_command_name(path: &str) -> String {
    // Remover el "/" inicial si existe
    let path = path.trim_start_matches('/');
    // Si hay query params, tomar solo la parte antes de "?"
    let cmd = path.split('?').next().unwrap_or(path);
    // Si está vacío o es solo "/", usar "root"
    if cmd.is_empty() {
        "root".to_string()
    } else {
        cmd.to_string()
    }
}

/// Encola la ejecución del `handler` en `pool`.
/// Registra t_enqueue y pasa información de timing al worker.
fn enqueue_on_pool(
    stream: &mut TcpStream,
    req: Request,
    handler: HandlerFn,
    pool: &WorkQueue,
    state: &Shared,
) -> std::io::Result<()> {
    let req_cloned = req.clone();
    let state_cloned = Arc::clone(state);
    let mut stream_clone = stream.try_clone()?; // cada tarea escribe en su propio handle
    
    // Extraer nombre del comando
    let cmd_name = extract_command_name(&req.path);
    
    // Registrar t_enqueue (momento en que se encola)
    let t_enqueue = now_ms_since_epoch();

    match pool.submit(Box::new(move || {
        // Registrar t_start (momento en que worker toma la tarea)
        let t_start = now_ms_since_epoch();
        let wait_ms = (t_start - t_enqueue) as u64;
        
        // Ejecutar handler
        let (status, _ctype, body) = handler(&state_cloned, &req_cloned);
        
        // Registrar t_end (momento en que termina la ejecución)
        let t_end = now_ms_since_epoch();
        let exec_ms = (t_end - t_start) as u64;
        
        // Registrar métricas por comando
        state_cloned.metrics.record_command_timing(&cmd_name, wait_ms, exec_ms);

        let reason = match status {
            200 => "OK",
            400 => "Bad Request",
            404 => "Not Found",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "OK",
        };

        let _ = write_response(
            &mut stream_clone,
            status,
            reason,
            &req_cloned.request_id,
            &body,
        );
    })) {
        Ok(()) => Ok(()),
        Err(Backpressure {
            retry_after_ms,
            queue,
        }) => {
            let body = format!(
                r#"{{"error":"backpressure","queue":"{}","retry_after_ms":{}}}"#,
                queue, retry_after_ms
            );
            write_response(
                stream,
                503,
                "Service Unavailable",
                &req.request_id,
                body.as_bytes(),
            )
        }
    }
}

/// Serializa una respuesta HTTP/1.0 mínima.
/// Cierra la conexión al terminar (semántica HTTP/1.0).
fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    req_id: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let headers = format!(
        "HTTP/1.0 {} {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         X-Request-Id: {}\r\n\
         \r\n",
        status,
        reason,
        body.len(),
        req_id
    );
    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}