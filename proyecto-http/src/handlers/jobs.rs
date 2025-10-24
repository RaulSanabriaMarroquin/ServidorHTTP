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

/// GET /jobs/submit?task=TASK&<params>&prio=priority
pub fn submit(state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let task_param = req.query.get("task");
    let default_prio = "normal".to_string();
    let prio_param = req.query.get("prio").unwrap_or(&default_prio);
    
    if task_param.is_none() {
        return bad_request("Parameter 'task' is required");
    }
    
    let task = task_param.unwrap();
    
    // Validar que la tarea sea soportada
    let supported_tasks = ["isprime", "fibonacci", "factor", "pi", "mandelbrot", "matrixmul"];
    if !supported_tasks.contains(&task.as_str()) {
        return bad_request(&format!("Unsupported task: {}. Supported: {:?}", task, supported_tasks));
    }
    
    // Parsear prioridad
    let priority = match prio_param.parse::<Priority>() {
        Ok(p) => p,
        Err(_) => return bad_request("Parameter 'prio' must be 'low', 'normal', or 'high'"),
    };
    
    // Extraer parámetros de la tarea (excluyendo 'task' y 'prio')
    let mut params = HashMap::new();
    for (key, value) in &req.query {
        if key != "task" && key != "prio" {
            params.insert(key.clone(), value.clone());
        }
    }
    
    // Validar parámetros según la tarea
    if let Err(msg) = validate_task_params(task, &params) {
        return bad_request(&msg);
    }
    
    // Encolar trabajo
    let job_id = state.job_store.submit(task.clone(), params, priority);
    
    let body = format!(
        r#"{{"job_id":"{}","status":"queued","task":"{}","priority":"{}"}}"#,
        job_id.0, task, prio_param
    );
    
    json_ok(body.into_bytes())
}

/// GET /jobs/status?id=JOBID
pub fn status(state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let id_param = req.query.get("id");
    
    if id_param.is_none() {
        return bad_request("Parameter 'id' is required");
    }
    
    let job_id = JobId(id_param.unwrap().clone());
    
    match state.job_store.status(&job_id) {
        Some(status_view) => {
            let body = format!(
                r#"{{"job_id":"{}","status":"{}","progress":{},"eta_ms":{}}}"#,
                job_id.0, status_view.status, status_view.progress, status_view.eta_ms
            );
            json_ok(body.into_bytes())
        },
        None => not_found(&format!("Job '{}' not found", job_id.0)),
    }
}

/// GET /jobs/result?id=JOBID
pub fn result(state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let id_param = req.query.get("id");
    
    if id_param.is_none() {
        return bad_request("Parameter 'id' is required");
    }
    
    let job_id = JobId(id_param.unwrap().clone());
    
    match state.job_store.result(&job_id) {
        Some(result) => json_ok(result.into_bytes()),
        None => {
            // Verificar si el trabajo existe pero no está completado
            match state.job_store.status(&job_id) {
                Some(status_view) => {
                    let body = format!(
                        r#"{{"error":"job_not_ready","status":"{}","progress":{}}}"#,
                        status_view.status, status_view.progress
                    );
                    bad_request(&body)
                },
                None => not_found(&format!("Job '{}' not found", job_id.0)),
            }
        }
    }
}

/// GET /jobs/cancel?id=JOBID
pub fn cancel(state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let id_param = req.query.get("id");
    
    if id_param.is_none() {
        return bad_request("Parameter 'id' is required");
    }
    
    let job_id = JobId(id_param.unwrap().clone());
    
    if state.job_store.cancel(&job_id) {
        let body = format!(
            r#"{{"job_id":"{}","status":"canceled","message":"Job canceled successfully"}}"#,
            job_id.0
        );
        json_ok(body.into_bytes())
    } else {
        // Verificar si el trabajo existe
        match state.job_store.status(&job_id) {
            Some(status_view) => {
                let body = format!(
                    r#"{{"error":"cannot_cancel","status":"{}","message":"Job cannot be canceled in current state"}}"#,
                    status_view.status
                );
                bad_request(&body)
            },
            None => not_found(&format!("Job '{}' not found", job_id.0)),
        }
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
        },
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
        },
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
                        return Err(format!("Parameter '{}' must be a valid positive integer", param));
                    }
                }
            }
        },
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
        },
        _ => return Err(format!("Unknown task: {}", task)),
    }
    
    Ok(())
}
