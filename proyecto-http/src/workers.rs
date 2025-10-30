// src/workers.rs
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::Shared;

/// Firma de cualquier handler ejecutable por un worker.
// (La dejamos por si la usas después en Sprint 3+)
pub type HandlerFn = fn(&Shared, &crate::core::Request) -> (u16, &'static str, Vec<u8>);

/// Tarea que procesa un worker.
pub type Task = Box<dyn FnOnce() + Send + 'static>;

#[derive(Clone, Debug)]
pub struct WorkerView {
    pub id: String,
    pub busy: bool,
}


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
    // NUEVO: estado por worker
    worker_busy: Vec<Arc<AtomicBool>>,
    worker_ids: Vec<String>,
}

impl WorkQueue {
    /// Crea la cola + lanza `workers` hilos consumidores.
    pub fn new(name: &'static str, max_depth: usize, workers: usize) -> Self {
        let (tx, rx) = mpsc::channel::<Task>();
        let rx_arc = Arc::new(Mutex::new(rx));
        let pending = Arc::new(Mutex::new(0usize));

        let mut handles = Vec::with_capacity(workers);
        let mut worker_busy = Vec::with_capacity(workers);
        let mut worker_ids = Vec::with_capacity(workers);
        
        for i in 0..workers {
    let rx_i = Arc::clone(&rx_arc);
    let pending_i = Arc::clone(&pending);

    // NUEVO: id y flag busy de este worker
    let wid = format!("{}-w{}", name, i);
    let busy_flag = Arc::new(AtomicBool::new(false));
    let busy_flag_thread = Arc::clone(&busy_flag);

    let h = thread::spawn(move || {
        loop {
            let task = {
                let lock = rx_i.lock().expect("rx poisoned");
                lock.recv()
            };
            match task {
                Ok(job) => {
                    // marcar ocupado
                    busy_flag_thread.store(true, Ordering::SeqCst);
                    job();
                    // desocupar
                    busy_flag_thread.store(false, Ordering::SeqCst);

                    if let Ok(mut p) = pending_i.lock() {
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

        // IMPORTANTE: registrar id y flag en los vectores
        worker_ids.push(wid);
        worker_busy.push(busy_flag);
        handles.push(h);
    }

        Self {
            name,
            tx,
            rx: rx_arc,
            pending,
            max_depth,
            workers: handles,
            worker_busy,
            worker_ids,
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
    /// NUEVO: vista de workers (id, busy)
    pub fn worker_views(&self) -> Vec<WorkerView> {
        let mut out = Vec::with_capacity(self.worker_ids.len());
        for (i, wid) in self.worker_ids.iter().enumerate() {
            let busy = self.worker_busy[i].load(Ordering::SeqCst);
            out.push(WorkerView {
                id: wid.clone(),
                busy,
            });
        }
        out
    }
}

/// Conjunto de colas/pools.
pub struct Pools {
    pub basic: WorkQueue,
    pub cpu: WorkQueue,
    pub io: WorkQueue,
}

// Defaults (hasta que los movamos a Config en Sprint 3)
const DEFAULT_WORKERS_BASIC: usize = 2;
const DEFAULT_WORKERS_CPU: usize = 4;
const DEFAULT_WORKERS_IO: usize = 4;

const DEFAULT_QUEUE_BASIC: usize = 64;
const DEFAULT_QUEUE_CPU: usize = 128;
const DEFAULT_QUEUE_IO: usize = 128;

    /// Crea los pools reales (por ahora ignoramos `state`, pero lo dejamos
    /// en la firma porque en Sprint 3 lo usaremos para métricas y config).
impl Pools {
    pub fn new(_state: &Shared) -> Self {
        Self {
            basic: WorkQueue::new("basic", 64, 2),
            cpu:   WorkQueue::new("cpu",   128, 4),
            io:    WorkQueue::new("io",    128, 4),
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

