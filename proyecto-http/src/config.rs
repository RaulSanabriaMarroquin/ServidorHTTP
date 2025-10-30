//! Definición y construcción de la configuración del servidor.

use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,

    pub workers_basic: usize,
    pub workers_cpu: usize,
    pub workers_io: usize,

    pub queue_basic: usize,
    pub queue_cpu: usize,
    pub queue_io: usize,

    // --- NUEVO: límites y timeouts para Jobs ---
    /// Máximo de trabajos encolados antes de aplicar backpressure (503).
    pub jobs_queue_max: usize,

    /// Timeouts de referencia por tipo de trabajo.
    pub timeout_cpu_ms: u64,
    pub timeout_io_ms: u64,

    /// Límite de concurrencia por tipo (aparte de pools globales)
    pub max_running_cpu_jobs: usize,
    pub max_running_io_jobs: usize,
}

impl Config {
    /// Lee la primera var de entorno que exista entre varias llaves (alias) y la parsea.
    fn first_env<T: FromStr>(keys: &[&str], default: T) -> T {
        for k in keys {
            if let Ok(val) = std::env::var(k) {
                if let Ok(parsed) = val.parse::<T>() {
                    return parsed;
                }
            }
        }
        default
    }

    /// Variables soportadas (todas opcionales; defaults entre paréntesis):
    /// - PORT | HTTP_PORT (8080)
    /// - WORKERS_BASIC | BASIC_WORKERS (2)
    /// - WORKERS_CPU   | CPU_WORKERS   (4)
    /// - WORKERS_IO    | IO_WORKERS    (4)
    /// - QUEUE_BASIC   | BASIC_QDEPTH  | BASIC_QUEUE (64)
    /// - QUEUE_CPU     | CPU_QDEPTH    | CPU_QUEUE   (128)
    /// - QUEUE_IO      | IO_QDEPTH     | IO_QUEUE    (128)
    /// - TIMEOUT_CPU_MS (60000)
    /// - TIMEOUT_IO_MS  (120000)
    pub fn from_env_or_default() -> Self {
        Self {
            port: Self::first_env(&["PORT", "HTTP_PORT"], 8080),

            workers_basic: Self::first_env(&["WORKERS_BASIC", "BASIC_WORKERS"], 2),
            workers_cpu:   Self::first_env(&["WORKERS_CPU",   "CPU_WORKERS"],   4),
            workers_io:    Self::first_env(&["WORKERS_IO",    "IO_WORKERS"],    4),

            queue_basic:   Self::first_env(&["QUEUE_BASIC", "BASIC_QDEPTH", "BASIC_QUEUE"], 64),
            queue_cpu:     Self::first_env(&["QUEUE_CPU",   "CPU_QDEPTH",   "CPU_QUEUE"],   128),
            queue_io:      Self::first_env(&["QUEUE_IO",    "IO_QDEPTH",    "IO_QUEUE"],    128),

            // --- NUEVOS campos inicializados ---
            jobs_queue_max:      Self::first_env(&["JOBS_QUEUE_MAX"], 1000),
            timeout_cpu_ms:      Self::first_env(&["TIMEOUT_CPU_MS"], 60_000),
            timeout_io_ms:       Self::first_env(&["TIMEOUT_IO_MS"],  120_000),
            max_running_cpu_jobs: Self::first_env(&["MAX_RUNNING_CPU_JOBS"], 2),
            max_running_io_jobs:  Self::first_env(&["MAX_RUNNING_IO_JOBS"],  4),
        }
    }
}
