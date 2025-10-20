//! Enrutador HTTP → “comandos” conocidos.
//!
//! Por ahora no definimos un enum Command completo porque en Sprint 0
//! solo queremos demostrar que el path se reconoce.
//!
//! La función clave aquí es `handle_early(path, req_id)`: si la ruta es conocida,
//! devolvemos un JSON “placeholder” (200 OK). Si no, `None` y el core responderá 404.

#[derive(Clone, Debug)]
pub struct Router;

impl Router {
    /// Construye el router (podríamos aquí registrar todas las rutas).
    pub fn new() -> Self { Self }

    /// Sprint 0: respuesta temprana si la ruta es conocida.
    /// Devuelve `Some(body_json)` si el path existe, o `None` si no existe.
    pub fn handle_early(&self, path: &str, req_id: &str) -> Option<String> {
        // Conjunto mínimo de rutas conocidas para probar el flujo end-to-end.
        // (Más tarde se ampliará a todas las del proyecto)
        const KNOWN: [&str; 8] = [
            "/status",
            "/timestamp",
            "/reverse",
            "/toupper",
            "/isprime",
            "/help",
            "/sleep",
            "/fibonacci",
        ];

        if KNOWN.contains(&path) {
            // JSON “placeholder” para ver que enruta:
            Some(format!(
                r#"{{"status":"routed","path":"{}","request_id":"{}"}}"#,
                path, req_id
            ))
        } else {
            None
        }
    }
}
