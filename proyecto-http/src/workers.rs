// src/workers.rs
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::core::Shared;

/// Firma de cualquier handler ejecutable por un worker.
// (La dejamos por si la usas después en Sprint 3+)
pub type HandlerFn = fn(&Shared, &crate::core::Request) -> (u16, &'static str, Vec<u8>);

/// Tarea que procesa un worker.
pub type Task = Box<dyn FnOnce() + Send + 'static>;

/// Error 503 por presión de cola.
#[derive(Debug)]
pub struct Backpressure {
    pub retry_after_ms: u64,
    pub queue: &'static str,
}

/// Foto simple para /status (si luego quieres exponerla).
#[derive(Clone, Debug)]
pub struct QueueSnapshot {
    pub name: &'static str,
    pub pending: usize,
    pub max_depth: usize,
    pub workers: usize,
}

/// Cola con límite de profundidad + pool de workers.
pub struct WorkQueue {
    pub name: &'static str,
    pub tx: mpsc::Sender<Task>,
    // Receiver compartido entre hilos via Arc<Mutex<...>>
    rx: Arc<Mutex<mpsc::Receiver<Task>>>,
    // Contador de tareas en cola/ejecución para hacer backpressure sencillo
    pending: Arc<Mutex<usize>>,
    pub max_depth: usize,
    pub workers: Vec<JoinHandle<()>>,
}

impl WorkQueue {
    /// Crea la cola + lanza `workers` hilos consumidores.
    pub fn new(name: &'static str, max_depth: usize, workers: usize) -> Self {
        let (tx, rx) = mpsc::channel::<Task>();
        let rx_arc = Arc::new(Mutex::new(rx));
        let pending = Arc::new(Mutex::new(0usize));

        let mut handles = Vec::with_capacity(workers);
        for _id in 0..workers {
            let rx_i = Arc::clone(&rx_arc);
            // cada job decrementará pending al terminar
            let pending_i = Arc::clone(&pending);

            let h = thread::spawn(move || {
                loop {
                    // bloquea hasta tener trabajo
                    let task = {
                        let lock = rx_i.lock().expect("rx poisoned");
                        lock.recv()
                    };
                    match task {
                        Ok(job) => {
                            // Ejecutar el trabajo
                            job();
                            // al terminar, --pending
                            if let Ok(mut p) = pending_i.lock() {
                                // saturating_sub por seguridad
                                *p = p.saturating_sub(1);
                            }
                        }
                        Err(_) => {
                            // Canal cerrado → fin worker
                            break;
                        }
                    }
                }
            });
            handles.push(h);
        }

        Self {
            name,
            tx,
            rx: rx_arc,
            pending,
            max_depth,
            workers: handles,
        }
    }

    /// Encola un trabajo respetando el límite de profundidad.
    pub fn submit(&self, task: Task) -> Result<(), Backpressure> {
        // chequeo y ++pending
        {
            let mut p = self.pending.lock().expect("pending poisoned");
            if *p >= self.max_depth {
                return Err(Backpressure {
                    retry_after_ms: 100,
                    queue: self.name,
                });
            }
            *p += 1;
        }

        // Envolvemos para decrementar pending al terminar (por si futuros productores no lo hacen)
        let _pending_i = Arc::clone(&self.pending);
        let wrapped = Box::new(move || {
            // Ejecuta la tarea original
            task();
            // (nota: normalmente ya se decrementa al terminar en el hilo consumidor;
            // dejamos este doble seguro comentado. Si prefieres centralizar el --pending
            // aquí, elimina el --pending del worker y descomenta este.)
            // if let Ok(mut p) = pending_i.lock() {
            //     *p = p.saturating_sub(1);
            // }
        }) as Task;

        // Enviar al canal
        self.tx
            .send(wrapped)
            .map_err(|_| Backpressure {
                retry_after_ms: 100,
                queue: self.name,
            })
    }

    /// Foto rápida del estado de la cola.
    pub fn snapshot(&self) -> QueueSnapshot {
        let p = self.pending.lock().map(|g| *g).unwrap_or(0);
        QueueSnapshot {
            name: self.name,
            pending: p,
            max_depth: self.max_depth,
            workers: self.workers.len(),
        }
    }
}

/// Conjunto de colas/pools.
pub struct Pools {
    pub basic: WorkQueue,
    pub cpu: WorkQueue,
    pub io: WorkQueue,
}

// Los valores por defecto ahora vienen de Config; dejamos las constantes
// previas eliminadas para evitar confusión y warnings.

    /// Crea los pools reales (por ahora ignoramos `state`, pero lo dejamos
    /// en la firma porque en Sprint 3 lo usaremos para métricas y config).
impl Pools {
    pub fn new(state: &Shared) -> Self {
        // Usa configuración en lugar de valores fijos
        let cfg = &state.cfg;
        Self {
            basic: WorkQueue::new("basic", cfg.queue_basic, cfg.workers_basic),
            cpu:   WorkQueue::new("cpu",   cfg.queue_cpu,   cfg.workers_cpu),
            io:    WorkQueue::new("io",    cfg.queue_io,    cfg.workers_io),
        }
    }
    /// Pools “dummy” para tests: sin workers y profundidad 0.
    pub fn new_dummy() -> Self {
        Self {
            basic: WorkQueue::new("basic_dummy", 0, 0),
            cpu:   WorkQueue::new("cpu_dummy",   0, 0),
            io:    WorkQueue::new("io_dummy",    0, 0),
        }
    }
}

