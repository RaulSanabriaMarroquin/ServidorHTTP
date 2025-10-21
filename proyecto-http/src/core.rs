//! Núcleo HTTP/1.0 (Sprint 0 y base para Sprint 1).
//!
//! Expone:
//! - `AppState`  : estado global (config, router, timestamps y métricas).
//! - `Shared`    : alias `Arc<AppState>` para compartir estado entre hilos.
//! - `Request`   : representación mínima de una petición HTTP (GET).
//! - `http_listen_loop()` : loop principal que acepta conexiones.
//! - `handle_connection()` : lee → parsea → rutea → escribe respuesta.
//!
//! Notas clave de diseño:
//! - HTTP/1.0 sin keep-alive: una petición por conexión y se cierra.
//! - Solo GET en Sprint 0/1. Otros métodos devuelven 501.
//! - La query `?a=1&b=2` se parsea a `HashMap<String, String>`.
//! - Métricas mínimas (`accepted`/`handled`) protegidas con Mutex (requisito del curso).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use crate::config::Config;
use crate::metrics::Metrics;  // Arc<Mutex<...>> por dentro (cumple Arc/Mutex)
use crate::router::Router;

/// Representa una petición HTTP simplificada para nuestros handlers.
/// - `method`      : "GET"
/// - `path`        : "/reverse"
/// - `query`       : {"text": "hola"}
/// - `http_version`: "HTTP/1.0" o "HTTP/1.1" (solo informativo por ahora)
/// - `request_id`  : id útil para trazabilidad / logs
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
    /// Construye el `Shared` listo para pasar a `http_listen_loop`.
    pub fn shared(cfg: Config, router: Router) -> Shared {
        let started_ms = now_ms_since_epoch();
        Arc::new(AppState {
            cfg,
            router,
            started_ms,
            metrics: Arc::new(Metrics::default()),
        })
    }
}

/// Inicia el listener HTTP/1.0 y atiende conexiones en hilos cortos.
/// - `bind(0.0.0.0:port)`
/// - `accept()` loop:
///     - incrementa métricas `accepted`
///     - spawn hilo: `handle_connection(...)`
///     - al terminar el hilo incrementa `handled`
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

/// Lee UNA petición (HTTP/1.0), la parsea y escribe la respuesta completa:
/// - Si no es GET → 501
/// - Si el router no reconoce la ruta → 404
/// - Si la reconoce → devuelve (status, content_type, body) y se escriben headers+body
fn handle_connection(state: &Shared, stream: &mut TcpStream, req_id: &str) -> std::io::Result<()> {
    // 1) Leer hasta 8KB (suficiente para Sprint 0/1).
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        // Cliente cerró inmediatamente o envió vacío.
        return Ok(());
    }
    let raw = String::from_utf8_lossy(&buf[..n]);

    // 2) Parsear la request-line: "GET /ruta?query HTTP/1.0"
    let mut lines = raw.lines();
    let req_line = lines.next().unwrap_or_default();
    let mut parts = req_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/");
    let httpver = parts.next().unwrap_or("HTTP/1.0").to_string();

    // 3) Solo soportamos GET por ahora.
    if method != "GET" {
        let body = format!(
            r#"{{"error":"not_implemented","method":"{}","request_id":"{}"}}"#,
            method, req_id
        );
        return write_full_response(
            stream,
            501,
            "application/json",
            body.as_bytes(),
            req_id,
        );
    }

    // 4) Separar path y query-string.
    let (path, query_map) = split_path_and_query(target);

    // 5) Armar `Request` para el router/handlers.
    let req = Request {
        method,
        path: path.to_string(),
        query: query_map,
        http_version: httpver,
        request_id: req_id.to_string(),
    };

    // 6) Ruteo: el router devuelve (status, content_type, body)
    let (status, content_type, body) = state.router.route(state, &req);

    // 7) Escribir respuesta completa y cerrar (HTTP/1.0)
    write_full_response(stream, status, content_type, &body, req_id)
}

/// Divide `"/ruta?x=1&y=2"` en:
/// - `"/ruta"`
/// - `HashMap{ "x"->"1", "y"->"2" }`
/// *No* decodifica URL (Sprint 1 simple); se puede agregar en sprints futuros.
fn split_path_and_query(target: &str) -> (&str, HashMap<String, String>) {
    let mut query_map = HashMap::new();
    let mut it = target.splitn(2, '?');
    let path = it.next().unwrap_or("/");
    if let Some(q) = it.next() {
        for pair in q.split('&') {
            if pair.is_empty() { continue; }
            let mut kv = pair.splitn(2, '=');
            let k = kv.next().unwrap_or("").to_string();
            let v = kv.next().unwrap_or("").to_string();
            if !k.is_empty() {
                query_map.insert(k, v);
            }
        }
    }
    (path, query_map)
}

/// Serializa una respuesta HTTP/1.0 completa:
/// - Status line  : "HTTP/1.0 200 OK"
/// - Headers      : Content-Type, Content-Length, X-Request-Id
/// - Body         : bytes
/// Cierra la conexión al terminar (semántica HTTP/1.0).
fn write_full_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    request_id: &str,
) -> std::io::Result<()> {
    let status_line = match status {
        200 => "HTTP/1.0 200 OK",
        400 => "HTTP/1.0 400 Bad Request",
        404 => "HTTP/1.0 404 Not Found",
        409 => "HTTP/1.0 409 Conflict",
        429 => "HTTP/1.0 429 Too Many Requests",
        500 => "HTTP/1.0 500 Internal Server Error",
        501 => "HTTP/1.0 501 Not Implemented",
        503 => "HTTP/1.0 503 Service Unavailable",
        _   => "HTTP/1.0 500 Internal Server Error",
    };

    // Headers mínimos y seguros para JSON/binary.
    let headers = format!(
        "{status_line}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {len}\r\n\
         X-Request-Id: {rid}\r\n\
         \r\n",
        status_line = status_line,
        content_type = content_type,
        len = body.len(),
        rid = request_id,
    );

    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}
