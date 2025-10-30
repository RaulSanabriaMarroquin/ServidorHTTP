//! Métricas del servidor usando Arc<Mutex<...>> (requerimiento del curso).
//!
//! Implementa métricas detalladas por comando según el enunciado:
//! - Tiempos de espera (wait_ms) y ejecución (exec_ms) separados
//! - Estadísticas por comando: count, avg, stddev, p50/p95/p99
//! - Colas y workers por comando

use std::sync::Mutex;
use std::collections::{VecDeque, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

/// Métricas de un comando específico
#[derive(Debug, Clone)]
pub struct CommandMetrics {
    pub wait_samples: VecDeque<u64>,  // wait_ms samples
    pub exec_samples: VecDeque<u64>,  // exec_ms samples
    pub max_samples: usize,
}

impl CommandMetrics {
    pub fn new(max_samples: usize) -> Self {
        Self {
            wait_samples: VecDeque::with_capacity(max_samples),
            exec_samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    pub fn add_wait_sample(&mut self, wait_ms: u64) {
        if self.wait_samples.len() >= self.max_samples {
            self.wait_samples.pop_front();
        }
        self.wait_samples.push_back(wait_ms);
    }

    pub fn add_exec_sample(&mut self, exec_ms: u64) {
        if self.exec_samples.len() >= self.max_samples {
            self.exec_samples.pop_front();
        }
        self.exec_samples.push_back(exec_ms);
    }

    pub fn count(&self) -> usize {
        self.exec_samples.len() // count basado en ejecuciones completadas
    }

    pub fn avg_wait(&self) -> f64 {
        if self.wait_samples.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.wait_samples.iter().sum();
        sum as f64 / self.wait_samples.len() as f64
    }

    pub fn avg_exec(&self) -> f64 {
        if self.exec_samples.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.exec_samples.iter().sum();
        sum as f64 / self.exec_samples.len() as f64
    }

    pub fn stddev_wait(&self) -> f64 {
        if self.wait_samples.len() < 2 {
            return 0.0;
        }
        let avg = self.avg_wait();
        let variance: f64 = self.wait_samples.iter()
            .map(|&x| {
                let diff = x as f64 - avg;
                diff * diff
            })
            .sum::<f64>() / (self.wait_samples.len() - 1) as f64;
        variance.sqrt()
    }

    pub fn stddev_exec(&self) -> f64 {
        if self.exec_samples.len() < 2 {
            return 0.0;
        }
        let avg = self.avg_exec();
        let variance: f64 = self.exec_samples.iter()
            .map(|&x| {
                let diff = x as f64 - avg;
                diff * diff
            })
            .sum::<f64>() / (self.exec_samples.len() - 1) as f64;
        variance.sqrt()
    }

    fn get_percentile(samples: &VecDeque<u64>, percentile: f64) -> u64 {
        if samples.is_empty() {
            return 0;
        }
        let mut sorted: Vec<u64> = samples.iter().cloned().collect();
        sorted.sort_unstable();
        let index = ((sorted.len() as f64 - 1.0) * percentile / 100.0) as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    pub fn p50_wait(&self) -> u64 {
        Self::get_percentile(&self.wait_samples, 50.0)
    }

    pub fn p95_wait(&self) -> u64 {
        Self::get_percentile(&self.wait_samples, 95.0)
    }

    pub fn p99_wait(&self) -> u64 {
        Self::get_percentile(&self.wait_samples, 99.0)
    }

    pub fn p50_exec(&self) -> u64 {
        Self::get_percentile(&self.exec_samples, 50.0)
    }

    pub fn p95_exec(&self) -> u64 {
        Self::get_percentile(&self.exec_samples, 95.0)
    }

    pub fn p99_exec(&self) -> u64 {
        Self::get_percentile(&self.exec_samples, 99.0)
    }
}

/// Estadísticas resumidas de un comando para JSON
#[derive(Debug, Clone)]
pub struct CommandStats {
    pub count: usize,
    pub avg_wait_ms: f64,
    pub avg_exec_ms: f64,
    pub stddev_wait_ms: f64,
    pub stddev_exec_ms: f64,
    pub p50_wait_ms: u64,
    pub p95_wait_ms: u64,
    pub p99_wait_ms: u64,
    pub p50_exec_ms: u64,
    pub p95_exec_ms: u64,
    pub p99_exec_ms: u64,
}

impl From<&CommandMetrics> for CommandStats {
    fn from(metrics: &CommandMetrics) -> Self {
        Self {
            count: metrics.count(),
            avg_wait_ms: metrics.avg_wait(),
            avg_exec_ms: metrics.avg_exec(),
            stddev_wait_ms: metrics.stddev_wait(),
            stddev_exec_ms: metrics.stddev_exec(),
            p50_wait_ms: metrics.p50_wait(),
            p95_wait_ms: metrics.p95_wait(),
            p99_wait_ms: metrics.p99_wait(),
            p50_exec_ms: metrics.p50_exec(),
            p95_exec_ms: metrics.p95_exec(),
            p99_exec_ms: metrics.p99_exec(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct MetricsState {
    pub accepted: u64,
    pub handled: u64,
    pub errors: u64,
    pub latency_samples: VecDeque<u64>, // Milisegundos (global, para compatibilidad)
    pub max_samples: usize,
    // Métricas por comando
    pub command_metrics: HashMap<String, CommandMetrics>,
}

impl MetricsState {
    pub fn new(max_samples: usize) -> Self {
        Self {
            accepted: 0,
            handled: 0,
            errors: 0,
            latency_samples: VecDeque::with_capacity(max_samples),
            max_samples,
            command_metrics: HashMap::new(),
        }
    }
    
    pub fn add_latency_sample(&mut self, latency_ms: u64) {
        if self.latency_samples.len() >= self.max_samples {
            self.latency_samples.pop_front();
        }
        self.latency_samples.push_back(latency_ms);
    }

    pub fn get_or_create_command_metrics(&mut self, cmd: &str) -> &mut CommandMetrics {
        self.command_metrics.entry(cmd.to_string())
            .or_insert_with(|| CommandMetrics::new(self.max_samples))
    }

    pub fn record_command_timing(&mut self, cmd: &str, wait_ms: u64, exec_ms: u64) {
        let cmd_metrics = self.get_or_create_command_metrics(cmd);
        cmd_metrics.add_wait_sample(wait_ms);
        cmd_metrics.add_exec_sample(exec_ms);
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

    pub fn get_all_command_stats(&self) -> HashMap<String, CommandStats> {
        self.command_metrics.iter()
            .map(|(cmd, metrics)| (cmd.clone(), CommandStats::from(metrics)))
            .collect()
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

    /// Registra una muestra de latencia (global, para compatibilidad).
    #[inline]
    pub fn record_latency(&self, latency_ms: u64) {
        if let Ok(mut st) = self.inner.lock() {
            st.add_latency_sample(latency_ms);
        }
    }

    /// Registra timing de un comando (wait_ms y exec_ms).
    #[inline]
    pub fn record_command_timing(&self, cmd: &str, wait_ms: u64, exec_ms: u64) {
        if let Ok(mut st) = self.inner.lock() {
            st.record_command_timing(cmd, wait_ms, exec_ms);
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

    /// Obtiene métricas detalladas incluyendo latencias (globales).
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

    /// Obtiene todas las métricas por comando.
    #[inline]
    pub fn get_all_command_stats(&self) -> HashMap<String, CommandStats> {
        if let Ok(st) = self.inner.lock() {
            st.get_all_command_stats()
        } else {
            HashMap::new()
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