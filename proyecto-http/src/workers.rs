//! Esqueleto para colas/pools y handlers (se implementará en sprints siguientes).
//!
//! La idea: el router decidirá si enviar un `WorkItem` a una `WorkQueue` (FIFO con
//! prioridad), y uno de los workers (threads) ejecutará el `Handler` del comando.

/// Prioridad para planificación en colas.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Low,
    Normal,
    High,
}

/// Unidad de trabajo que terminará ejecutando un handler.
/// (En el futuro tendrá: request parseado, cmd, prioridad, timestamps…)
#[allow(dead_code)]
pub struct WorkItem;

/// Contrato de un handler (por comando).
/// (Definiremos la firma cuando tengamos nuestro tipo `Response` compartido.)
#[allow(dead_code)]
pub trait Handler {
    // fn handle(&self, item: WorkItem) -> Response;
}

/// Resultado de intentar encolar un trabajo.
/// - Immediate: se resolvió inline y devolvemos la `Response`.
/// - Enqueued: lo tomará un worker y devolvemos un `job_id`.
/// - Backpressure: cola saturada → 503 y retry_after_ms.
#[allow(dead_code)]
pub enum EnqueueOutcome {
    Immediate(/* Response */),
    Enqueued { job_id: String },
    Backpressure { retry_after_ms: u64 },
}

/// Interfaz de una cola con prioridad y límites.
/// (La implementación real usará Mutex/Condvar o canales mpsc).
#[allow(dead_code)]
pub trait WorkQueue {
    // fn submit(&self, item: WorkItem, prio: Priority) -> EnqueueOutcome;
}
