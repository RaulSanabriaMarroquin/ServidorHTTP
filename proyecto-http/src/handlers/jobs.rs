//! Handlers para el sistema de Jobs: /jobs/submit, /jobs/status, /jobs/result, /jobs/cancel
//!
//! Todos los handlers siguen la misma firma:
//!   (state: &Shared, req: &Request) -> (status, content_type, body_bytes)

use crate::core::{Request, Shared};
use crate::jobs::{JobId, Priority};
use std::collections::HashMap;



fn json_ok(bytes: Vec<u8>) -> (u16, &'static str, Vec<u8>) {
    (200, "application/json", bytes)
}

fn bad_request(msg: &str) -> (u16, &'static str, Vec<u8>) {
    (
        400,
        "application/json",
        format!(r#"{{"error":"bad_request","detail":"{}"}}"#, msg).into_bytes(),
    )
}

fn not_found(msg: &str) -> (u16, &'static str, Vec<u8>) {
    (
        404,
        "application/json",
        format!(r#"{{"error":"not_found","detail":"{}"}}"#, msg).into_bytes(),
    )
}

/// Si ?mode=job, encola el trabajo y devuelve {job_id,...}. Si no, retorna None.
/// - `task`: nombre lógico de la tarea (ej.: "isprime", "matrixmul", "sortfile"...)
/// - `allow_keys`: lista de claves de query que se copian como params del job
pub fn maybe_enqueue_job(
    state: &Shared,
    req: &Request,
    task: &str,
    allow_keys: &[&str],
) -> Option<(u16, &'static str, Vec<u8>)> {
    let mode = req.query.get("mode").map(|s| s.as_str()).unwrap_or("direct");
    if mode != "job" { return None; }

    // prioridad (opcional): prio=low|normal|high
    let prio = req.query.get("prio").map(|s| s.as_str()).unwrap_or("normal");
    let priority = match prio.parse::<Priority>() {
        Ok(p) => p,
        Err(_) => return Some(bad_request("Parameter 'prio' must be 'low'|'normal'|'high'")),
    };

    // backpressure
    let pending = state.job_store.pending_len();
    let max_queue = state.cfg.jobs_queue_max;
    if pending >= max_queue {
        let body = format!(r#"{{"error":"overloaded","retry_after_ms":{}}}"#, 2000);
        return Some((503, "application/json", body.into_bytes()));
    }

    // construir params
    let mut params = HashMap::new();
    for &k in allow_keys {
        if let Some(v) = req.query.get(k) {
            params.insert(k.to_string(), v.to_string());
        }
    }

    // encolar
    let job_id = state.job_store.submit(task.to_string(), params, priority);
    let body = format!(r#"{{"job_id":"{}","status":"queued","task":"{}","priority":"{}"}}"#, job_id.0, task, prio);
    Some(json_ok(body.into_bytes()))
}

/// GET /jobs/submit?task=TASK&<params>&prio=priority
pub fn submit(state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let task = match req.query.get("task") {
        Some(t) => t.to_string(),
        None => return bad_request("Parameter 'task' is required"),
    };
    let default_prio = "normal".to_string();
    let prio_param = req.query.get("prio").unwrap_or(&default_prio);

    // backpressure
    let pending = state.job_store.pending_len();
    let max_queue = state.cfg.jobs_queue_max;
    if pending >= max_queue {
        let body = format!(r#"{{"error":"overloaded","retry_after_ms":{}}}"#, 2000);
        return (503, "application/json", body.into_bytes());
    }

    // validar task soportada
    let supported = ["isprime","fibonacci","factor","pi","mandelbrot","matrixmul",
                     "sortfile","wordcount","grep","compress","hashfile"];
    if !supported.contains(&task.as_str()) {
        return bad_request(&format!("Unsupported task: {}. Supported: {:?}", task, supported));
    }

    // prioridad
    let prio_raw = req.query.get("prio").map(|s| s.as_str()).unwrap_or("normal");
    let priority = match prio_raw.parse::<Priority>() {
        Ok(p) => p,
        Err(_) => return bad_request("Parameter 'prio' must be 'low'|'normal'|'high'"),
    };

    // params = query - {task, prio}
    let mut params = HashMap::new();
    for (k,v) in &req.query {
        if k != "task" && k != "prio" { params.insert(k.clone(), v.clone()); }
    }

    // (opcional) validación mínima por tarea (usa tu función si prefieres)
    // ...

    let id = state.job_store.submit(task.clone(), params, priority);
    let body = format!(r#"{{"job_id":"{}","status":"queued","task":"{}","priority":"{}"}}"#, id.0, task, prio_raw);
    json_ok(body.into_bytes())
}


/// GET /jobs/status?id=JOBID
pub fn status(state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
     eprintln!("[jobs/status] req_id={} query={:?}", req.request_id, req.query);

    let id = match req.query.get("id") {
        Some(x) if !x.is_empty() => JobId(x.clone()),
        _ => return (
            400, "application/json",
            br#"{"error":"bad_request","detail":"Parameter 'id' is required"}"#.to_vec()
        ),
    };

    match state.job_store.status(&id) {
        Some(view) => {
            // Construye JSON seguro sin format strings
            let body = serde_json::json!({
                "job_id": id.0,
                "status": view.status.to_string(),  // usa tu impl Display de JobStatus
                "progress": view.progress,
                "eta_ms": view.eta_ms
            });
            (200, "application/json", body.to_string().into_bytes())
        }
        None => {
            let body = serde_json::json!({
                "error": "not_found",
                "detail": format!("Job '{}' not found", id.0)
            });
            (404, "application/json", body.to_string().into_bytes())
        }
    }
}

/// GET /jobs/result?id=JOBID
pub fn result(state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let id = match req.query.get("id") { Some(x) => JobId(x.clone()), None => return bad_request("Parameter 'id' is required") };
    if let Some(res) = state.job_store.result(&id) {
        return json_ok(res.into_bytes());
    }
    match state.job_store.status(&id) {
        Some(v) => bad_request(&format!(r#"{{"error":"job_not_ready","status":"{}","progress":{}}}"#, v.status, v.progress)),
        None => not_found(&format!("Job '{}' not found", id.0)),
    }
}

/// GET /jobs/cancel?id=JOBID
pub fn cancel(state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let id = match req.query.get("id") { Some(x) => JobId(x.clone()), None => return bad_request("Parameter 'id' is required") };
    if state.job_store.cancel(&id) {
        let b = format!(r#"{{"job_id":"{}","status":"canceled"}}"#, id.0);
        return json_ok(b.into_bytes());
    }
    match state.job_store.status(&id) {
        Some(v) => bad_request(&format!(r#"{{"error":"cannot_cancel","status":"{}"}}"#, v.status)),
        None => not_found(&format!("Job '{}' not found", id.0)),
    }
}

/// Valida los parámetros de una tarea específica
fn validate_task_params(task: &str, params: &HashMap<String, String>) -> Result<(), String> {
    match task {
        "isprime" | "fibonacci" | "factor" => {
            if !params.contains_key("n") {
                return Err("Parameter 'n' is required".to_string());
            }
            if let Some(n_str) = params.get("n") {
                if n_str.parse::<u64>().is_err() {
                    return Err("Parameter 'n' must be a valid positive integer".to_string());
                }
            }
        }
        "pi" => {
            if !params.contains_key("digits") {
                return Err("Parameter 'digits' is required".to_string());
            }
            if let Some(digits_str) = params.get("digits") {
                if let Ok(digits) = digits_str.parse::<u32>() {
                    if digits == 0 || digits > 1000 {
                        return Err("Parameter 'digits' must be between 1 and 1000".to_string());
                    }
                } else {
                    return Err("Parameter 'digits' must be a valid positive integer".to_string());
                }
            }
        }
        "mandelbrot" => {
            let required_params = ["width", "height", "max_iter"];
            for param in &required_params {
                if !params.contains_key(*param) {
                    return Err(format!("Parameter '{}' is required", param));
                }
            }
            for param in &required_params {
                if let Some(value) = params.get(*param) {
                    if value.parse::<u32>().is_err() {
                        return Err(format!(
                            "Parameter '{}' must be a valid positive integer",
                            param
                        ));
                    }
                }
            }
        }
        "matrixmul" => {
            if !params.contains_key("size") {
                return Err("Parameter 'size' is required".to_string());
            }
            if let Some(size_str) = params.get("size") {
                if let Ok(size) = size_str.parse::<usize>() {
                    if size == 0 || size > 1000 {
                        return Err("Parameter 'size' must be between 1 and 1000".to_string());
                    }
                } else {
                    return Err("Parameter 'size' must be a valid positive integer".to_string());
                }
            }
        }
        _ => return Err(format!("Unknown task: {}", task)),
    }
    Ok(())
}
