//! Sistema de Jobs completo: gestión de trabajos asíncronos con persistencia.
//!
//! Implementa:
//! - Encolado de trabajos con prioridades
//! - Seguimiento de estado y progreso
//! - Persistencia efímera en archivo
//! - Cancelación de trabajos
//! - Timeouts configurables

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;
use std::io::{BufRead, BufReader};
use serde_json;
use std::thread;
use std::time::Duration;
use std::sync::mpsc;
use crate::core::{Shared, Request,AppState};

/// Estados posibles de un trabajo
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Error,
    Canceled,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "op")]
enum JournalEntry {
    #[serde(rename = "new")]
    New { job: Job },
    #[serde(rename = "update")]
    Update {
        id: String,
        status: Option<JobStatus>,
        progress: Option<u8>,
        result: Option<String>,
        error: Option<String>,
        started_at: Option<u128>,
        completed_at: Option<u128>,
    },
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Queued => write!(f, "queued"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Done => write!(f, "done"),
            JobStatus::Error => write!(f, "error"),
            JobStatus::Canceled => write!(f, "canceled"),
        }
    }
}

/// Identificador único de trabajo
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct JobId(pub String);

/// Información de un trabajo
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Job {
    pub id: JobId,
    pub task: String,
    pub params: HashMap<String, String>,
    pub status: JobStatus,
    pub progress: u8,
    pub created_at: u128,
    pub started_at: Option<u128>,
    pub completed_at: Option<u128>,
    pub result: Option<String>,
    pub error: Option<String>,
    pub priority: Priority,
    pub timeout_ms: u64,
}

/// Prioridades de trabajo
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Priority::Low),
            "normal" => Ok(Priority::Normal),
            "high" => Ok(Priority::High),
            _ => Err(format!("Invalid priority: {}", s)),
        }
    }
}

/// Vista resumida del estado de un trabajo
#[derive(Debug, Clone, serde::Serialize)]
pub struct JobStatusView {
    pub status: JobStatus,
    pub progress: u8,
    pub eta_ms: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum JobKind { Cpu, Io, Unknown }

impl JobKind {
    pub fn from_task(task: &str) -> JobKind {
        match task {
            // CPU-bound
            "isprime" | "fibonacci" | "factor" | "pi" | "mandelbrot" | "matrixmul" => JobKind::Cpu,
            // IO-bound
            "sortfile" | "wordcount" | "grep" | "compress" | "hashfile" => JobKind::Io,
            _ => JobKind::Unknown,
        }
    }
}

/// Almacenamiento de trabajos con persistencia
pub struct JobStore {
    jobs: Arc<Mutex<HashMap<JobId, Job>>>,
    journal_path: String,
}

impl JobStore {
    pub fn new() -> Self {
        let store = Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            journal_path: "data/jobs.journal".to_string(),
        };

        // Crear directorio si no existe
        std::fs::create_dir_all("data").unwrap_or_default();

        // Recuperar trabajos del journal
        let _ = store.recover_from_journal();

        store
    }

    fn append_journal(&self, entry: &JournalEntry) {
        let line = serde_json::to_string(entry).unwrap_or_else(|_| "{}".to_string());
        use std::io::Write;
        let _ = std::fs::create_dir_all("data");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.journal_path)
        {
            let _ = writeln!(f, "{line}");
        }
    }

    /// Encola un nuevo trabajo
    pub fn submit(&self, task: String, params: HashMap<String, String>, priority: Priority) -> JobId {
        let id = JobId(format!("job-{}", now_ms()));
        let timeout_ms = match task.as_str() {
            "pi" | "matrixmul" | "mandelbrot" | "factor" | "isprime" => self.timeout_cpu_ms(),
            _ => self.timeout_io_ms(),
        };
        let job = Job {
            id: id.clone(),
            task,
            params,
            status: JobStatus::Queued,
            progress: 0,
            created_at: now_ms(),
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
            priority,
            timeout_ms,
        };
        {
            self.jobs.lock().unwrap().insert(id.clone(), job.clone());
        }
        self.append_journal(&JournalEntry::New { job });
        id
    }

    fn timeout_cpu_ms(&self) -> u64 {
        read_cfg_u64("TIMEOUT_CPU_MS", 60000)
    }
    fn timeout_io_ms(&self) -> u64 {
        read_cfg_u64("TIMEOUT_IO_MS", 30000)
    }

}

// función libre para leer configuración
fn read_cfg_u64(key: &str, def: u64) -> u64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(def)
}

// -------------------------
// Segundo impl JobStore: consultas/actualizaciones y persistencia
// -------------------------
impl JobStore {
    /// Cuenta trabajos encolados (para backpressure)
    pub fn pending_len(&self) -> usize {
        let jobs = self.jobs.lock().unwrap();
        jobs.values().filter(|j| j.status == JobStatus::Queued).count()
    }

    /// Helper para compatibilidad con llamadas a `persist_job(id)`
    fn persist_job(&self, id: &JobId) {
        if let Some(job) = self.jobs.lock().unwrap().get(id).cloned() {
            self.persist_update(&job);
        }
    }

    /// Obtiene el estado de un trabajo
    pub fn status(&self, id: &JobId) -> Option<JobStatusView> {
        let jobs = self.jobs.lock().unwrap();
        jobs.get(id).map(|job| {
            let eta_ms = match job.status {
                JobStatus::Queued => 5000, // Estimación conservadora
                JobStatus::Running => {
                    if let Some(started) = job.started_at {
                        let elapsed = now_ms() - started;
                        let remaining_progress = 100 - job.progress;
                        if job.progress > 0 {
                            ((elapsed * remaining_progress as u128) / job.progress as u128) as u64
                        } else {
                            10000 // Estimación si no hay progreso
                        }
                    } else {
                        5000
                    }
                }
                JobStatus::Done | JobStatus::Error | JobStatus::Canceled => 0,
            };

            JobStatusView {
                status: job.status,
                progress: job.progress,
                eta_ms,
            }
        })
    }

    /// Obtiene el resultado de un trabajo completado
    pub fn result(&self, id: &JobId) -> Option<String> {
        let jobs = self.jobs.lock().unwrap();
        jobs.get(id).and_then(|job| match job.status {
            JobStatus::Done => job.result.clone(),
            JobStatus::Error => Some(format!(
                r#"{{"error":"{}","message":"Job failed"}}"#,
                job.error.as_deref().unwrap_or("unknown")
            )),
            _ => None,
        })
    }

    pub fn count_running_kind(&self, kind: JobKind) -> usize {
        let map = self.jobs.lock().unwrap();
        map.values()
            .filter(|j| j.status == JobStatus::Running && JobKind::from_task(&j.task) == kind)
            .count()
    }

    /// Copia defensiva del job (para que el dispatcher lea el snapshot actual)
    pub fn get_job(&self, id: &JobId) -> Option<Job> {
        self.jobs.lock().unwrap().get(id).cloned()
    }

    /// Intenta cancelar un trabajo
    pub fn cancel(&self, id: &JobId) -> bool {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(id) {
            match job.status {
                JobStatus::Queued => {
                    job.status = JobStatus::Canceled;
                    job.completed_at = Some(now_ms());
                    self.persist_job(id);
                    true
                }
                JobStatus::Running => {
                    // Marcamos como cancelado; el worker debe verificar periódicamente
                    job.status = JobStatus::Canceled;
                    job.completed_at = Some(now_ms());
                    self.persist_job(id);
                    true
                }
                _ => false, // No se puede cancelar trabajos terminados
            }
        } else {
            false
        }
    }

    /// Actualiza el progreso de un trabajo
    pub fn update_progress(&self, id: &JobId, progress: u8) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(id) {
            job.progress = progress.min(100);
            self.persist_job(id);
        }
    }

    /// Marca un trabajo como iniciado
    pub fn start_job(&self, id: &JobId) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(id) {
            job.status = JobStatus::Running;
            job.started_at = Some(now_ms());
            self.persist_job(id);
        }
    }

    /// Marca un trabajo como completado
    pub fn complete_job(&self, id: &JobId, result: String) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(id) {
            job.status = JobStatus::Done;
            job.progress = 100;
            job.completed_at = Some(now_ms());
            job.result = Some(result);
            self.persist_job(id);
        }
    }

    /// Marca un trabajo como fallido
    pub fn fail_job(&self, id: &JobId, error: String) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get_mut(id) {
            job.status = JobStatus::Error;
            job.completed_at = Some(now_ms());
            job.error = Some(error);
            self.persist_job(id);
        }
    }

    /// Obtiene trabajos pendientes ordenados por prioridad (FIFO por prioridad)
    pub fn get_pending_jobs(&self) -> Vec<JobId> {
        let jobs = self.jobs.lock().unwrap();
        let mut pending: Vec<_> = jobs
            .values()
            .filter(|job| job.status == JobStatus::Queued)
            .map(|job| (job.priority, job.created_at, job.id.clone()))
            .collect();

        pending.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        pending.into_iter().map(|(_, _, id)| id).collect()
    }

    /// Limpia trabajos antiguos completados
    pub fn cleanup_old_jobs(&self, max_age_ms: u128) {
        let cutoff = now_ms() - max_age_ms;
        let mut jobs = self.jobs.lock().unwrap();
        let to_remove: Vec<_> = jobs
            .iter()
            .filter(|(_, job)| {
                job.status == JobStatus::Done
                    || job.status == JobStatus::Error
                    || job.status == JobStatus::Canceled
            })
            .filter(|(_, job)| job.completed_at.map_or(false, |completed| completed < cutoff))
            .map(|(id, _)| id.clone())
            .collect();

        for id in to_remove {
            jobs.remove(&id);
        }

        // Reescribir journal
        self.write_journal();
    }

    /// Persiste actualización (op=update) en JSONL
    fn persist_update(&self, job: &Job) {
        self.append_journal(&JournalEntry::Update {
            id: job.id.0.clone(),
            status: Some(job.status),
            progress: Some(job.progress),
            result: job.result.clone(),
            error: job.error.clone(),
            started_at: job.started_at,
            completed_at: job.completed_at,
        });
    }

    /// Recupera trabajos del journal
    fn recover_from_journal(&self) -> std::io::Result<()> {
        if !std::path::Path::new(&self.journal_path).exists() {
            return Ok(());
        }
        let file = fs::File::open(&self.journal_path)?;
        let reader = BufReader::new(file);
        let mut map: HashMap<JobId, Job> = HashMap::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<JournalEntry>(&line) {
                match entry {
                    JournalEntry::New { job } => {
                        map.insert(job.id.clone(), job);
                    }
                    JournalEntry::Update {
                        id,
                        status,
                        progress,
                        result,
                        error,
                        started_at,
                        completed_at,
                    } => {
                        if let Some(j) = map.get_mut(&JobId(id.clone())) {
                            if let Some(s) = status {
                                j.status = s;
                            }
                            if let Some(p) = progress {
                                j.progress = p;
                            }
                            if let Some(r) = result {
                                j.result = Some(r);
                            }
                            if let Some(e) = error {
                                j.error = Some(e);
                            }
                            if let Some(st) = started_at {
                                j.started_at = Some(st);
                            }
                            if let Some(cp) = completed_at {
                                j.completed_at = Some(cp);
                            }
                        }
                    }
                }
            }
        }

        // Jobs que quedaron "running" en un reinicio -> marcarlos como error "restarted"
        for j in map.values_mut() {
            if j.status == JobStatus::Running {
                j.status = JobStatus::Error;
                j.error = Some("restarted".into());
                j.completed_at = Some(now_ms());
            }
        }

        *self.jobs.lock().unwrap() = map;
        Ok(())
    }

    /// Escribe todo el journal (estado actual)
    fn write_journal(&self) {
        let jobs = self.jobs.lock().unwrap();
        let mut journal_content = String::new();
        for job in jobs.values() {
            if let Ok(entry) = serde_json::to_string(job) {
                journal_content.push_str(&entry);
                journal_content.push('\n');
            }
        }
        let _ = fs::write(&self.journal_path, journal_content);
    }
}

/// Almacenamiento global de trabajos
lazy_static::lazy_static! {
    static ref JOB_STORE: JobStore = JobStore::new();
}

/// Timestamp actual en ms
fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Ejecuta un trabajo según su tipo
pub fn execute_job(job: &Job, state: &Shared) -> String {
    let to_json = |code: u16, body: Vec<u8>, fallback: &str| {
        if code == 200 {
            String::from_utf8(body).unwrap_or_else(|_| fallback.into())
        } else {
            fallback.into()
        }
    };

    match job.task.as_str() {
        // ---------------- CPU ----------------
        "isprime" => {
            let mut q = std::collections::HashMap::new();
            if let Some(n) = job.params.get("n") { q.insert("n".into(), n.clone()); }
            if let Some(m) = job.params.get("method") { q.insert("method".into(), m.clone()); }
            let (c, _t, b) = crate::handlers::cpu::isprime(state, &Request{
                method:"GET".into(), path:"/isprime".into(), query:q,
                http_version:"HTTP/1.0".into(), request_id:"job".into()
            });
            to_json(c, b, r#"{"error":"isprime_failed"}"#)
        }
        "fibonacci" => {
            let mut q = std::collections::HashMap::new();
            if let Some(n)=job.params.get("n"){ q.insert("n".into(), n.clone()); }
            let (c,_t,b)=crate::handlers::basic::fibonacci(state, &Request{
                method:"GET".into(), path:"/fibonacci".into(), query:q,
                http_version:"HTTP/1.0".into(), request_id:"job".into()
            });
            to_json(c,b,r#"{"error":"fibonacci_failed"}"#)
        }
        "factor" => {
            let mut q=std::collections::HashMap::new();
            if let Some(n)=job.params.get("n"){ q.insert("n".into(), n.clone()); }
            let (c,_t,b)=crate::handlers::cpu::factor(state, &Request{
                method:"GET".into(), path:"/factor".into(), query:q,
                http_version:"HTTP/1.0".into(), request_id:"job".into()
            });
            to_json(c,b,r#"{"error":"factor_failed"}"#)
        }
        "pi" => {
            let mut q=std::collections::HashMap::new();
            if let Some(d)=job.params.get("digits"){ q.insert("digits".into(), d.clone()); }
            let (c,_t,b)=crate::handlers::cpu::pi(state, &Request{
                method:"GET".into(), path:"/pi".into(), query:q,
                http_version:"HTTP/1.0".into(), request_id:"job".into()
            });
            to_json(c,b,r#"{"error":"pi_failed"}"#)
        }
        "mandelbrot" => {
            let mut q=std::collections::HashMap::new();
            for k in ["width","height","max_iter"] {
                if let Some(v)=job.params.get(k){ q.insert(k.to_string(), v.clone()); }
            }
            let (c,_t,b)=crate::handlers::cpu::mandelbrot(state, &Request{
                method:"GET".into(), path:"/mandelbrot".into(), query:q,
                http_version:"HTTP/1.0".into(), request_id:"job".into()
            });
            to_json(c,b,r#"{"error":"mandelbrot_failed"}"#)
        }
        "matrixmul" => {
            let mut q=std::collections::HashMap::new();
            if let Some(s)=job.params.get("size"){ q.insert("size".into(), s.clone()); }
            if let Some(s)=job.params.get("seed"){ q.insert("seed".into(), s.clone()); }
            let (c,_t,b)=crate::handlers::cpu::matrixmul(state, &Request{
                method:"GET".into(), path:"/matrixmul".into(), query:q,
                http_version:"HTTP/1.0".into(), request_id:"job".into()
            });
            to_json(c,b,r#"{"error":"matrixmul_failed"}"#)
        }

        // ---------------- IO ----------------
        "sortfile" => {
            let mut q=std::collections::HashMap::new();
            for k in ["name","algo"] {
                if let Some(v)=job.params.get(k){ q.insert(k.to_string(), v.clone()); }
            }
            let (c,_t,b)=crate::handlers::io::sortfile(state, &Request{
                method:"GET".into(), path:"/sortfile".into(), query:q,
                http_version:"HTTP/1.0".into(), request_id:"job".into()
            });
            to_json(c,b,r#"{"error":"sortfile_failed"}"#)
        }
        "wordcount" => {
            let mut q=std::collections::HashMap::new();
            if let Some(v)=job.params.get("name"){ q.insert("name".into(), v.clone()); }
            let (c,_t,b)=crate::handlers::io::wordcount(state, &Request{
                method:"GET".into(), path:"/wordcount".into(), query:q,
                http_version:"HTTP/1.0".into(), request_id:"job".into()
            });
            to_json(c,b,r#"{"error":"wordcount_failed"}"#)
        }
        "grep" => {
            let mut q=std::collections::HashMap::new();
            for k in ["name","pattern"] {
                if let Some(v)=job.params.get(k){ q.insert(k.to_string(), v.clone()); }
            }
            let (c,_t,b)=crate::handlers::io::grep(state, &Request{
                method:"GET".into(), path:"/grep".into(), query:q,
                http_version:"HTTP/1.0".into(), request_id:"job".into()
            });
            to_json(c,b,r#"{"error":"grep_failed"}"#)
        }
        "compress" => {
            let mut q=std::collections::HashMap::new();
            for k in ["name","codec"] {
                if let Some(v)=job.params.get(k){ q.insert(k.to_string(), v.clone()); }
            }
            let (c,_t,b)=crate::handlers::io::compress(state, &Request{
                method:"GET".into(), path:"/compress".into(), query:q,
                http_version:"HTTP/1.0".into(), request_id:"job".into()
            });
            to_json(c,b,r#"{"error":"compress_failed"}"#)
        }
        "hashfile" => {
            let mut q=std::collections::HashMap::new();
            for k in ["name","algo"] {
                if let Some(v)=job.params.get(k){ q.insert(k.to_string(), v.clone()); }
            }
            let (c,_t,b)=crate::handlers::io::hashfile(state, &Request{
                method:"GET".into(), path:"/hashfile".into(), query:q,
                http_version:"HTTP/1.0".into(), request_id:"job".into()
            });
            to_json(c,b,r#"{"error":"hashfile_failed"}"#)
        }

        _ => r#"{"error":"unknown_task"}"#.into(),
    }
}

// ---- Funciones auxiliares de cómputo ----
fn is_prime_number(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    let sqrt_n = (n as f64).sqrt() as u64;
    for i in (3..=sqrt_n).step_by(2) {
        if n % i == 0 {
            return false;
        }
    }
    true
}

fn fibonacci_calc(n: u32) -> Option<u64> {
    if n > 93 {
        return None;
    }
    if n == 0 {
        return Some(0);
    }
    if n == 1 {
        return Some(1);
    }
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    for _ in 2..=n {
        let temp = a + b;
        a = b;
        b = temp;
    }
    Some(b)
}

fn factorize(n: u64) -> Vec<Vec<u64>> {
    let mut factors = Vec::new();
    let mut num = n;

    if num % 2 == 0 {
        let mut count = 0;
        while num % 2 == 0 {
            num /= 2;
            count += 1;
        }
        factors.push(vec![2, count]);
    }

    let mut i = 3;
    while i * i <= num {
        if num % i == 0 {
            let mut count = 0;
            while num % i == 0 {
                num /= i;
                count += 1;
            }
            factors.push(vec![i, count]);
        }
        i += 2;
    }

    if num > 1 {
        factors.push(vec![num, 1]);
    }

    factors
}

/// Lanza el hilo supervisor del JobStore: planifica, respeta límites por tipo y aplica timeouts.
/// Llamar una vez en el arranque del servidor (p.ej. en main.rs).
pub fn spawn_dispatcher(state: Arc<AppState>) {
    thread::spawn(move || {
        loop {
            // 1) tomar pendientes ya ordenados por prioridad/FIFO
            let pending = state.job_store.get_pending_jobs();

            for job_id in pending {
                // 2) validar límites de concurrencia por tipo de tarea
                let job = match state.job_store.get_job(&job_id) {
                    Some(j) => j,
                    None => continue,
                };

                let kind = JobKind::from_task(&job.task);
                let (limit, running_now) = match kind {
                    JobKind::Cpu => (
                        state.cfg.max_running_cpu_jobs,
                        state.job_store.count_running_kind(JobKind::Cpu),
                    ),
                    JobKind::Io => (
                        state.cfg.max_running_io_jobs,
                        state.job_store.count_running_kind(JobKind::Io),
                    ),
                    JobKind::Unknown => (
                        1,
                        state.job_store.count_running_kind(JobKind::Unknown),
                    ),
                };

                if running_now >= limit {
                    // ya alcanzamos el cupo de este tipo; intenta en el siguiente ciclo
                    continue;
                }

                // 3) marcar inicio y lanzar ejecución aislada
                state.job_store.start_job(&job_id);
                let st = state.clone();

                thread::spawn(move || {
                    // canal para capturar resultado sin bloquear
                    let (tx, rx) = mpsc::channel::<String>();
                    let jid = job_id.clone();

                    // Clonamos el job para el hilo de ejecución y conservamos datos para este scope
                    let job_for_exec = match st.job_store.get_job(&jid) {
                        Some(j) => j,
                        None => {
                            st.job_store.fail_job(&jid, "job_not_found_after_start".into());
                            return;
                        }
                    };
                    let timeout_ms = job_for_exec.timeout_ms; // usamos el timeout aquí fuera
                    let job_snapshot = job_for_exec.clone();  // copia para mover al hilo worker

                    // Hilo que ejecuta el trabajo (consume la copia)
                    let st_exec = st.clone();
                    thread::spawn(move || {
                        let out = execute_job(&job_snapshot, &st_exec);
                        let _ = tx.send(out);
                    });

                    // 4) esperar con timeout según el job
                    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
                        Ok(result_json) => {
                            // si alguien lo canceló en medio, no sobreescribas
                            if let Some(jcurr) = st.job_store.get_job(&jid) {
                                if jcurr.status == JobStatus::Canceled {
                                    return;
                                }
                            }
                            st.job_store.complete_job(&jid, result_json);
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            st.job_store.fail_job(&jid, "timeout".into());
                        }
                        Err(_) => {
                            st.job_store.fail_job(&jid, "worker_crashed".into());
                        }
                    }
                });
            }

            // 5) ritmo del scheduler
            thread::sleep(Duration::from_millis(150));
        }
    });
}