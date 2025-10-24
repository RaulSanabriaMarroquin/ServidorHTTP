//! Métricas del servidor usando Arc<Mutex<...>> (requerimiento del curso).
//!
//! Elegimos un Mutex porque:
//! - Cumple el criterio de "sincronización explícita" solicitado.
//! - Es suficiente para Sprint 1 (contadores simples).
//! - Deja la puerta abierta a extender las métricas (p50/p95/p99, histos) sin
//!   pelearse con atomics múltiples.
//!
//! Nota: El Mutex es muy corto (se bloquea solo para sumar o leer), por lo que
//! el impacto en rendimiento es negligible en esta etapa.

use std::sync::Mutex;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Default, Clone)]
pub struct MetricsState {
    pub accepted: u64,
    pub handled: u64,
    pub errors: u64,
    pub latency_samples: VecDeque<u64>, // Milisegundos
    pub max_samples: usize,
}

impl MetricsState {
    pub fn new(max_samples: usize) -> Self {
        Self {
            accepted: 0,
            handled: 0,
            errors: 0,
            latency_samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }
    
    pub fn add_latency_sample(&mut self, latency_ms: u64) {
        if self.latency_samples.len() >= self.max_samples {
            self.latency_samples.pop_front();
        }
        self.latency_samples.push_back(latency_ms);
    }
    
    pub fn get_percentile(&self, percentile: f64) -> u64 {
        if self.latency_samples.is_empty() {
            return 0;
        }
        
        let mut sorted_samples: Vec<u64> = self.latency_samples.iter().cloned().collect();
        sorted_samples.sort_unstable();
        
        let index = ((sorted_samples.len() as f64 - 1.0) * percentile / 100.0) as usize;
        sorted_samples[index.min(sorted_samples.len() - 1)]
    }
    
    pub fn get_average_latency(&self) -> f64 {
        if self.latency_samples.is_empty() {
            return 0.0;
        }
        
        let sum: u64 = self.latency_samples.iter().sum();
        sum as f64 / self.latency_samples.len() as f64
    }
}

/// Wrapper con Mutex para acceso thread-safe.
#[derive(Debug)]
pub struct Metrics {
    pub inner: Mutex<MetricsState>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new(1000) // Mantener últimos 1000 samples
    }
}

impl Metrics {
    pub fn new(max_samples: usize) -> Self {
        Self {
            inner: Mutex::new(MetricsState::new(max_samples)),
        }
    }

    /// +1 a "accepted" (connection aceptada).
    #[inline]
    pub fn inc_accepted(&self) {
        if let Ok(mut st) = self.inner.lock() {
            st.accepted += 1;
        }
    }

    /// +1 a "handled" (request totalmente atendida).
    #[inline]
    pub fn inc_handled(&self) {
        if let Ok(mut st) = self.inner.lock() {
            st.handled += 1;
        }
    }

    /// +1 a "errors" (request con error).
    #[inline]
    pub fn inc_errors(&self) {
        if let Ok(mut st) = self.inner.lock() {
            st.errors += 1;
        }
    }

    /// Registra una muestra de latencia.
    #[inline]
    pub fn record_latency(&self, latency_ms: u64) {
        if let Ok(mut st) = self.inner.lock() {
            st.add_latency_sample(latency_ms);
        }
    }

    /// Copia los contadores actuales.
    #[inline]
    pub fn snapshot(&self) -> (u64, u64) {
        if let Ok(st) = self.inner.lock() {
            (st.accepted, st.handled)
        } else {
            (0, 0)
        }
    }

    /// Obtiene métricas detalladas incluyendo latencias.
    #[inline]
    pub fn detailed_snapshot(&self) -> DetailedMetrics {
        if let Ok(st) = self.inner.lock() {
            DetailedMetrics {
                accepted: st.accepted,
                handled: st.handled,
                errors: st.errors,
                avg_latency_ms: st.get_average_latency(),
                p50_latency_ms: st.get_percentile(50.0),
                p95_latency_ms: st.get_percentile(95.0),
                p99_latency_ms: st.get_percentile(99.0),
                sample_count: st.latency_samples.len(),
            }
        } else {
            DetailedMetrics::default()
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct DetailedMetrics {
    pub accepted: u64,
    pub handled: u64,
    pub errors: u64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub sample_count: usize,
}

/// Función auxiliar para obtener timestamp actual en milisegundos
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
