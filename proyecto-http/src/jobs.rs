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

use crate::core::Shared;

/// Estados posibles de un trabajo
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Error,
    Canceled,
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
    
    /// Encola un nuevo trabajo
    pub fn submit(&self, task: String, params: HashMap<String, String>, priority: Priority) -> JobId {
        let id = JobId(format!("job-{}", now_ms()));
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
            timeout_ms: 60000, // 60 segundos por defecto
        };
        
        {
            let mut jobs = self.jobs.lock().unwrap();
            jobs.insert(id.clone(), job);
        }
        
        self.persist_job(&id);
        id
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
                },
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
        jobs.get(id).and_then(|job| {
            match job.status {
                JobStatus::Done => job.result.clone(),
                JobStatus::Error => Some(format!(
                    r#"{{"error":"{}","message":"Job failed"}}"#,
                    job.error.as_deref().unwrap_or("unknown")
                )),
                _ => None,
            }
        })
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
                },
                JobStatus::Running => {
                    // Para trabajos en ejecución, marcamos como cancelado
                    // pero el worker debe verificar periódicamente
                    job.status = JobStatus::Canceled;
                    job.completed_at = Some(now_ms());
                    self.persist_job(id);
                    true
                },
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
    
    /// Obtiene trabajos pendientes ordenados por prioridad
    pub fn get_pending_jobs(&self) -> Vec<JobId> {
        let jobs = self.jobs.lock().unwrap();
        let mut pending: Vec<_> = jobs.values()
            .filter(|job| job.status == JobStatus::Queued)
            .map(|job| (job.priority, job.created_at, job.id.clone()))
            .collect();
        
        // Ordenar por prioridad (descendente) y luego por tiempo de creación
        pending.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        
        pending.into_iter().map(|(_, _, id)| id).collect()
    }
    
    /// Limpia trabajos antiguos completados
    pub fn cleanup_old_jobs(&self, max_age_ms: u128) {
        let cutoff = now_ms() - max_age_ms;
        let mut jobs = self.jobs.lock().unwrap();
        let to_remove: Vec<_> = jobs.iter()
            .filter(|(_, job)| {
                job.status == JobStatus::Done || 
                job.status == JobStatus::Error || 
                job.status == JobStatus::Canceled
            })
            .filter(|(_, job)| {
                job.completed_at.map_or(false, |completed| completed < cutoff)
            })
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in to_remove {
            jobs.remove(&id);
        }
        
        // Reescribir journal
        self.write_journal();
    }
    
    /// Persiste un trabajo en el journal
    fn persist_job(&self, id: &JobId) {
        let jobs = self.jobs.lock().unwrap();
        if let Some(job) = jobs.get(id) {
            let journal_entry = serde_json::to_string(job).unwrap_or_else(|_| "{}".to_string());
            let _ = fs::write(&self.journal_path, journal_entry + "\n");
        }
    }
    
    /// Recupera trabajos del journal
    fn recover_from_journal(&self) -> std::io::Result<()> {
        if !std::path::Path::new(&self.journal_path).exists() {
            return Ok(());
        }
        
        let file = fs::File::open(&self.journal_path)?;
        let reader = BufReader::new(file);
        let mut jobs = self.jobs.lock().unwrap();
        
        for line in reader.lines() {
            let line = line?;
            if let Ok(job) = serde_json::from_str::<Job>(&line) {
                jobs.insert(job.id.clone(), job);
            }
        }
        
        Ok(())
    }
    
    /// Escribe todo el journal
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

/// Función auxiliar para obtener timestamp actual
fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Ejecuta un trabajo según su tipo
pub fn execute_job(job: &Job, _state: &Shared) -> String {
    match job.task.as_str() {
        "isprime" => {
            if let Some(n_str) = job.params.get("n") {
                if let Ok(n) = n_str.parse::<u64>() {
                    let is_prime = is_prime_number(n);
                    serde_json::json!({
                        "n": n,
                        "is_prime": is_prime,
                        "method": "division"
                    }).to_string()
                } else {
                    r#"{"error":"invalid_parameter"}"#.to_string()
                }
            } else {
                r#"{"error":"missing_parameter"}"#.to_string()
            }
        },
        "fibonacci" => {
            if let Some(n_str) = job.params.get("n") {
                if let Ok(n) = n_str.parse::<u32>() {
                    if let Some(result) = fibonacci_calc(n) {
                        serde_json::json!({
                            "n": n,
                            "fibonacci": result
                        }).to_string()
                    } else {
                        r#"{"error":"calculation_error"}"#.to_string()
                    }
                } else {
                    r#"{"error":"invalid_parameter"}"#.to_string()
                }
            } else {
                r#"{"error":"missing_parameter"}"#.to_string()
            }
        },
        "factor" => {
            if let Some(n_str) = job.params.get("n") {
                if let Ok(n) = n_str.parse::<u64>() {
                    let factors = factorize(n);
                    serde_json::json!({
                        "n": n,
                        "factors": factors
                    }).to_string()
                } else {
                    r#"{"error":"invalid_parameter"}"#.to_string()
                }
            } else {
                r#"{"error":"missing_parameter"}"#.to_string()
            }
        },
        _ => r#"{"error":"unknown_task"}"#.to_string(),
    }
}

// Funciones auxiliares reutilizadas
fn is_prime_number(n: u64) -> bool {
    if n < 2 { return false; }
    if n == 2 { return true; }
    if n % 2 == 0 { return false; }
    
    let sqrt_n = (n as f64).sqrt() as u64;
    for i in (3..=sqrt_n).step_by(2) {
        if n % i == 0 { return false; }
    }
    true
}

fn fibonacci_calc(n: u32) -> Option<u64> {
    if n > 93 { return None; }
    if n == 0 { return Some(0); }
    if n == 1 { return Some(1); }
    
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