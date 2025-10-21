//! Métricas del servidor usando Arc<Mutex<...>> (requerimiento del curso).
//!
//! Elegimos un Mutex porque:
//! - Cumple el criterio de “sincronización explícita” solicitado.
//! - Es suficiente para Sprint 1 (contadores simples).
//! - Deja la puerta abierta a extender las métricas (p50/p95/p99, histos) sin
//!   pelearse con atomics múltiples.
//!
//! Nota: El Mutex es muy corto (se bloquea solo para sumar o leer), por lo que
//! el impacto en rendimiento es negligible en esta etapa.

use std::sync::Mutex;

#[derive(Debug, Default, Clone)]
pub struct MetricsState {
    pub accepted: u64,
    pub handled:  u64,
}

/// Wrapper con Mutex para acceso thread-safe.
#[derive(Debug, Default)]
pub struct Metrics {
    pub inner: Mutex<MetricsState>,
}

impl Metrics {
    /// +1 a “accepted” (connection aceptada).
    #[inline]
    pub fn inc_accepted(&self) {
        if let Ok(mut st) = self.inner.lock() {
            st.accepted += 1;
        }
    }

    /// +1 a “handled” (request totalmente atendida).
    #[inline]
    pub fn inc_handled(&self) {
        if let Ok(mut st) = self.inner.lock() {
            st.handled += 1;
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
}
