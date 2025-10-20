//! Definición y construcción de la configuración del servidor.
//!
//! En Sprint 0 mantenemos valores por defecto (puerto 8080).
//! En sprints siguientes, `from_env_or_default()` leerá variables de entorno y/o flags CLI.

#[derive(Clone, Debug)]
pub struct Config {
    /// Puerto TCP donde escuchará el servidor (p.ej. 8080).
    pub port: u16,
}

impl Config {
    /// Construye la config desde env/CLI o usa defaults.
    /// Sprint 0: devolvemos el puerto 8080.
    pub fn from_env_or_default() -> Self {
        Self { port: 8080 }
    }
}