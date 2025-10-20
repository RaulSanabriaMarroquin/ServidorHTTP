//! Núcleo HTTP/1.0 (mínimo viable en Sprint 0).
//!
//! Define:
//! - `AppState` (estado global: config, router, started_ms).
//! - `Shared = Arc<AppState>` (alias).
//! - `http_listen_loop()` (bucle principal del listener).
//! - `handle_connection()` (lee → parsea → consulta router → responde).
//!
//! Este archivo NO resuelve endpoints reales todavía; solo arma el flujo.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;
use crate::router::Router;

/// Estado global del servidor.
/// (En próximos sprints agregaremos métricas/pools/job-store aquí)
#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,      // configuración del servidor (puerto, etc.)
    pub router: Router,   // enrutador HTTP
    pub started_ms: u128, // timestamp de arranque (ms since epoch)
}

/// Alias conveniente para compartir estado por referencia
/// contada (`Arc`) entre hilos/funciones.
pub type Shared = Arc<AppState>;


impl AppState {
    /// Crea el `Shared` (Arc<AppState>) con config, router y timestamp de arranque.
    pub fn shared(cfg: Config, router: Router) -> Shared {
        let started_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        Arc::new(AppState { cfg, router, started_ms })
    }
}
/// Bucle del listener HTTP/1.0:
/// - bind(0.0.0.0:port)
/// - accept() conexiones
/// - para cada conexión, `handle_connection`
pub fn http_listen_loop(state: &Shared) -> std::io::Result<()> {
    let addr = format!("0.0.0.0:{}", state.cfg.port);
    let listener = TcpListener::bind(&addr)?;
    println!("[INFO] listening on http://{} (HTTP/1.0)", addr);

    // Generador sencillo de IDs de request para X-Request-Id.
    let mut next_id: u64 = 1;

    // Bucle de aceptación bloqueante (modelo 1:1 conexión→manejo inline).
    // En sprints futuros, haremos spawn o thread-pool.
    for stream in listener.incoming() {
        match stream {
            Ok(mut s) => {
                let req_id = format!("req-{}", next_id);
                next_id += 1;
                if let Err(e) = handle_connection(&state, &mut s, &req_id) {
                    eprintln!("[WARN] connection error ({}): {}", req_id, e);
                }
            }
            Err(e) => eprintln!("[WARN] accept error: {e}"),
        }
    }
    Ok(())
}

/// Manejo de UNA conexión TCP (HTTP/1.0, sin keep-alive):
/// 1) lee hasta 8KB
/// 2) parsea línea de request (método, target, versión)
/// 3) separa path vs query (por ahora ignoramos query)
/// 4) consulta al router; si None → 404; si Some(body) → 200
fn handle_connection(state: &Shared, stream: &mut TcpStream, req_id: &str) -> std::io::Result<()> {
    // Leer hasta 8KB del request (suficiente para Sprint 0).
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        // Cliente cerró inmediatamente o no envió nada.
        return Ok(());
    }
    let raw = String::from_utf8_lossy(&buf[..n]);

    // Parseo muy simple de la request-line: "GET /ruta?query HTTP/1.0"
    let mut lines = raw.lines();
    let req_line = lines.next().unwrap_or_default();
    let mut parts = req_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    let _httpver = parts.next().unwrap_or("");

    // Validamos que sea GET (Sprint 0 solo GET). Si no, 501 Not Implemented.
    if method != "GET" {
        let body = format!(
            r#"{{"error":"not_implemented","method":"{}","request_id":"{}"}}"#,
            method, req_id
        );
        return write_response(stream, 501, "Not Implemented", req_id, body.as_bytes());
    }

    // Separar path vs query. (Guardamos solo el path aquí.)
    let path = target.split('?').next().unwrap_or("/");

    // Preguntar al router si reconoce la ruta
    if let Some(body) = state.router.handle_early(path, req_id) {
        write_response(stream, 200, "OK", req_id, body.as_bytes())
    } else {
        let body = format!(
            r#"{{"error":"not_found","path":"{}","request_id":"{}"}}"#,
            path, req_id
        );
        write_response(stream, 404, "Not Found", req_id, body.as_bytes())
    }
}

/// Serializa una respuesta HTTP/1.0 mínima:
/// Status-Line, algunos headers (JSON + Content-Length + X-Request-Id) y body.
/// NOTA: Cerramos la conexión al terminar (HTTP/1.0; no keep-alive).
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
