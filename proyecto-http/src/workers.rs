// src/workers.rs
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::sync::atomic::{AtomicBool,AtomicUsize, Ordering}; 

use crate::core::Shared;

/// Firma de cualquier handler ejecutable por un worker.
// (La dejamos por si la usas después en Sprint 3+)
pub type HandlerFn = fn(&Shared, &crate::core::Request) -> (u16, &'static str, Vec<u8>);

/// Tarea que procesa un worker.
pub type Task = Box<dyn FnOnce() + Send + 'static>;

#[derive(Clone, Debug, serde::Serialize)]
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
    pub queued: usize,    // NUEVO
    pub running: usize,   // NUEVO
    pub max_depth: usize,
    pub workers: usize,
}

/// Cola con límite de profundidad + pool de workers.
pub struct WorkQueue {
    pub name: &'static str,
    pub tx: mpsc::Sender<Task>,
    // Receiver compartido entre hilos via Arc<Mutex<...>>
    _rx: Arc<Mutex<mpsc::Receiver<Task>>>,
    // Contador de tareas en cola/ejecución para hacer backpressure sencillo
    pending: Arc<Mutex<usize>>,
    pub max_depth: usize,
    pub workers: Vec<JoinHandle<()>>,
    // NUEVO: estado por worker
    worker_busy: Vec<Arc<AtomicBool>>,
    worker_ids: Vec<String>,
    running: Arc<AtomicUsize>,
}

impl WorkQueue {
    /// Crea la cola + lanza `workers` hilos consumidores.
    pub fn new(name: &'static str, max_depth: usize, workers: usize) -> Self {
        let (tx, rx) = mpsc::channel::<Task>();
        let rx_arc = Arc::new(Mutex::new(rx));
        let running = Arc::new(AtomicUsize::new(0));
        let pending = Arc::new(Mutex::new(0usize));

        let mut handles = Vec::with_capacity(workers);
        let mut worker_busy = Vec::with_capacity(workers);
        let mut worker_ids = Vec::with_capacity(workers);

        for i in 0..workers {
            let rx_i = Arc::clone(&rx_arc);
            let pending_i = Arc::clone(&pending);
            let running_i = Arc::clone(&running);

            // id y flag busy de este worker
            let wid = format!("{}-w{}", name, i);
            let busy_flag = Arc::new(AtomicBool::new(false));
            let busy_flag_thread = Arc::clone(&busy_flag);
            let wid_for_thread = wid.clone();

            let h = std::thread::Builder::new().name(wid.clone()).spawn(move || {
                loop {
                    let task = {
                        let lock = rx_i.lock().expect("rx poisoned");
                        lock.recv()
                    };
                    match task {
                        Ok(job) => {
                            // marcar ocupado
                            busy_flag_thread.store(true, Ordering::SeqCst);
                            running_i.fetch_add(1, Ordering::SeqCst);  // ++running

                            // Ejecutar blindado contra panic
                            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                job();
                            }));

                            running_i.fetch_sub(1, Ordering::SeqCst);  // --running
                            // desocupar (siempre)
                            busy_flag_thread.store(false, Ordering::SeqCst);
                            if let Ok(mut p) = pending_i.lock() {
                                *p = p.saturating_sub(1);
                            }

                            if let Err(e) = res {
                                eprintln!("[worker {}] handler panicked: {:?}", wid_for_thread, e);
                                // El hilo sigue vivo.
                            }
                        }
                        Err(_) => {
                            // Canal cerrado → fin worker
                            eprintln!("[worker {}] channel closed; exiting", wid_for_thread);
                            break;
                        }
                    }
                }
            }).expect("spawn worker");

            // Registrar id y flag en los vectores
            worker_ids.push(wid);
            worker_busy.push(busy_flag);
            handles.push(h);
        }

        Self {
            name,
            tx,
            _rx: rx_arc,
            pending,
            running,
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
        let r = self.running.load(Ordering::SeqCst);
        let q = p.saturating_sub(r);  // queued reales
        QueueSnapshot {
            name: self.name,
            pending: p,
            queued: q,
            running: r,
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

