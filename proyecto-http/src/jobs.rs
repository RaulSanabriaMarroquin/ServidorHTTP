//! Contratos del Job Manager (stub en Sprint 0).
//!
//! Implementaremos un journal simple (persistencia efímera) y APIs /jobs/*
//! en sprints siguientes.

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Error,
    Canceled,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct JobId(pub String);

/// Vista resumida del estado de un job (para /jobs/status).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct JobStatusView {
    pub status: JobStatus,
    pub progress: u8,  // 0..=100
    pub eta_ms: u64,
}

/// Almacenamiento/gestor de jobs (se implementará luego).
#[allow(dead_code)]
pub trait JobStore {
    // fn submit(&self, item: WorkItem) -> JobId;
    // fn status(&self, id: &JobId) -> JobStatusView;
    // fn result(&self, id: &JobId) -> Option<Response>;
    // fn cancel(&self, id: &JobId) -> bool;
    // fn recover_from_journal(&self) -> std::io::Result<()>;
}
