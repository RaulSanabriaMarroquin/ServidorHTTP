//! Handlers básicos: /status, /timestamp, /reverse, /toupper,
//! y básicos extra: /fibonacci, /isprime, /sleep, /help, + 404.
//!
//! Todos los handlers siguen la misma firma:
//!   (state: &Shared, req: &Request) -> (status, content_type, body_bytes)

use crate::core::{now_ms_since_epoch, Request, Shared};
use std::collections::HashMap;
use serde_json::json;

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
fn not_found_json(path: &str) -> (u16, &'static str, Vec<u8>) {
    (
        404,
        "application/json",
        format!(r#"{{"error":"not_found","path":"{}"}}"#, path).into_bytes(),
    )
}

/// GET /simulate?seconds=s&task=name
/// Simula trabajo real durante ~s segundos realizando cómputo (hashes) para no bloquear
/// con sleep puro. Limita el máximo a 15s.
pub fn simulate(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let default_seconds = "1".to_string();
    let seconds_str = req.query.get("seconds").unwrap_or(&default_seconds);
    let task_name = req.query.get("task").cloned().unwrap_or_else(|| "cpu_hash".to_string());

    let seconds: u64 = match seconds_str.parse::<u64>() {
        Ok(s) if s > 0 => s.min(15),
        _ => return bad_request("Parameter 'seconds' must be a positive integer"),
    };

    // Trabajo real: calcular hashes sobre un buffer para ~seconds segundos
    let start = now_ms_since_epoch();
    let deadline = start + (seconds as u128) * 1000;
    let mut iterations: u64 = 0;
    let mut bytes_processed: u64 = 0;

    // Buffer determinístico pequeño para no consumir memoria en exceso
    let mut data = vec![0u8; 64 * 1024]; // 64KiB
    for i in 0..data.len() {
        data[i] = (i as u8).wrapping_mul(31);
    }

    use sha2::{Digest, Sha256};
    while now_ms_since_epoch() < deadline {
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let _ = hasher.finalize();
        iterations += 1;
        bytes_processed += data.len() as u64;
    }

    let elapsed = now_ms_since_epoch() - start;
    let body = format!(
        r#"{{"task":"{}","seconds_requested":{},"elapsed_ms":{},"iterations":{},"bytes_processed":{}}}"#,
        task_name, seconds, elapsed, iterations, bytes_processed
    );
    json_ok(body.into_bytes())
}

/// GET /loadtest?tasks=n&sleep=ms
/// Lanza `n` tareas ligeras en paralelo que duermen `sleep` milisegundos cada una.
/// Devuelve tiempo total y throughput aproximado. Limita n a 2000 y sleep a 15000ms.
pub fn loadtest(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let default_tasks = "10".to_string();
    let default_sleep = "10".to_string();

    let tasks_s = req.query.get("tasks").unwrap_or(&default_tasks);
    let sleep_s = req.query.get("sleep").unwrap_or(&default_sleep);

    let tasks: usize = match tasks_s.parse::<usize>() {
        Ok(v) if v > 0 => v.min(2000),
        _ => return bad_request("Parameter 'tasks' must be a positive integer"),
    };
    let sleep_ms: u64 = match sleep_s.parse::<u64>() {
        Ok(v) => v.min(15000),
        _ => return bad_request("Parameter 'sleep' must be a non-negative integer"),
    };

    let start = now_ms_since_epoch();

    let mut handles = Vec::with_capacity(tasks);
    for _ in 0..tasks {
        handles.push(std::thread::spawn({
            let dur = std::time::Duration::from_millis(sleep_ms);
            move || {
                std::thread::sleep(dur);
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }

    let elapsed = now_ms_since_epoch() - start;
    let throughput = if elapsed > 0 { (tasks as u128 * 1000) / elapsed } else { 0 };

    let body = format!(
        r#"{{"tasks":{},"sleep_ms":{},"elapsed_ms":{},"throughput_tps":{}}}"#,
        tasks, sleep_ms, elapsed, throughput
    );
    json_ok(body.into_bytes())
}

/// GET /status
pub fn status(state: &Shared, _req: &Request) -> (u16, &'static str, Vec<u8>) {
    let (accepted, handled) = state.metrics.snapshot();

    // NUEVO: snapshots de colas
    let qb = state.pools.basic.snapshot();
    let qc = state.pools.cpu.snapshot();
    let qi = state.pools.io.snapshot();

    // NUEVO: detalle de workers por pool (id/busy)
    let workers_detail = json!({
        "basic": state.pools.basic.worker_views(),
        "cpu":   state.pools.cpu.worker_views(),
        "io":    state.pools.io.worker_views(),
    });


    let body = format!(
        r#"{{
  "status":"ok",
  "port":{port},
  "pid":{pid},
  "uptime_ms":{uptime},
  "metrics":{{"accepted":{acc},"handled":{hdl}}},
  "workers_detail": {workers_detail},
  "queues":[
        {{"name":"{qb_name}","queued":{qb_queued},"running":{qb_running},"pending":{qb_pending},"max_depth":{qb_max},"workers":{qb_workers}}},
    {{"name":"{qc_name}","queued":{qc_queued},"running":{qc_running},"pending":{qc_pending},"max_depth":{qc_max},"workers":{qc_workers}}},
    {{"name":"{qi_name}","queued":{qi_queued},"running":{qi_running},"pending":{qi_pending},"max_depth":{qi_max},"workers":{qi_workers}}}
  ],
  "config":{{
    "workers":{{"basic":{w_basic},"cpu":{w_cpu},"io":{w_io}}}, 
    "queues":{{"basic":{q_basic},"cpu":{q_cpu},"io":{q_io}}},
    "timeouts_ms":{{"cpu":{t_cpu},"io":{t_io}}}
  }}
}}"#,
        port = state.cfg.port,
        pid = std::process::id(),
        uptime = now_ms_since_epoch().saturating_sub(state.started_ms),
        acc = accepted,
        hdl = handled,

        qb_name = qb.name, qb_queued = qb.queued, qb_pending = qb.pending, qb_running = qb.running, qb_max = qb.max_depth, qb_workers = qb.workers,
        qc_name = qc.name, qc_queued = qc.queued, qc_running = qc.running, qc_pending = qc.pending, qc_max = qc.max_depth, qc_workers = qc.workers,
        qi_name = qi.name, qi_queued = qi.queued, qi_running = qi.running, qi_pending = qi.pending, qi_max = qi.max_depth, qi_workers = qi.workers,

        w_basic = state.cfg.workers_basic, w_cpu = state.cfg.workers_cpu, w_io = state.cfg.workers_io,
        q_basic = state.cfg.queue_basic,   q_cpu = state.cfg.queue_cpu,   q_io = state.cfg.queue_io,
        t_cpu   = state.cfg.timeout_cpu_ms, t_io = state.cfg.timeout_io_ms,
        workers_detail = workers_detail.to_string(),
    );

    json_ok(body.into_bytes())
}
/// GET /timestamp
pub fn timestamp(_state: &Shared, _req: &Request) -> (u16, &'static str, Vec<u8>) {
    json_ok(format!(r#"{{"unix_ms":{}}}"#, now_ms_since_epoch()).into_bytes())
}

/// GET /reverse?text=hola
pub fn reverse(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    match req.query.get("text") {
        Some(s) => {
            let rev: String = s.chars().rev().collect();
            json_ok(format!(r#"{{"input":"{}","output":"{}"}}"#, s, rev).into_bytes())
        }
        None => bad_request("missing_param: text"),
    }
}

/// GET /toupper?text=AbCd
pub fn toupper(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    match req.query.get("text") {
        Some(s) => json_ok(
            format!(r#"{{"input":"{}","output":"{}"}}"#, s, s.to_uppercase()).into_bytes(),
        ),
        None => bad_request("missing_param: text"),
    }
}

/// GET /random?count=n&min=a&max=b
pub fn random(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let default_count = "5".to_string();
    let default_min = "1".to_string();
    let default_max = "100".to_string();
    
    let count_param = req.query.get("count").unwrap_or(&default_count);
    let min_param = req.query.get("min").unwrap_or(&default_min);
    let max_param = req.query.get("max").unwrap_or(&default_max);
    
    let count = match count_param.parse::<u32>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'count' must be a valid positive integer"),
    };
    
    let min_val = match min_param.parse::<i32>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'min' must be a valid integer"),
    };
    
    let max_val = match max_param.parse::<i32>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'max' must be a valid integer"),
    };
    
    if count == 0 || count > 1000 {
        return bad_request("Parameter 'count' must be between 1 and 1000");
    }
    
    if min_val >= max_val {
        return bad_request("Parameter 'min' must be less than 'max'");
    }
    
    // Generar números aleatorios simples
    let mut numbers = Vec::new();
    for i in 0..count {
        let seed = now_ms_since_epoch() + i as u128;
        let range = max_val - min_val + 1;
        let random_num = min_val + ((seed % range as u128) as i32);
        numbers.push(random_num);
    }
    
    let numbers_json = serde_json::to_string(&numbers).unwrap_or_else(|_| "[]".to_string());
    let body = format!(
        r#"{{"count":{},"min":{},"max":{},"numbers":{}}}"#,
        count, min_val, max_val, numbers_json
    );
    
    json_ok(body.into_bytes())
}

/// GET /hash?text=someinput
pub fn hash(_state:&Shared, req:&Request)->(u16,&'static str,Vec<u8>){
    use sha2::{Sha256, Digest};
    use hex::encode as hex_encode;

    let text = req.query.get("text").cloned().unwrap_or_else(|| "hello world".to_string());
    if text.is_empty() { return bad_request("Parameter 'text' cannot be empty"); }

    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let out_hex = hex_encode(hasher.finalize());

    json_ok(format!(r#"{{"text":"{}","algo":"sha256","hash":"{}"}}"#, text, out_hex).into_bytes())
}


/*// GET /simulate?seconds=s&task=name
pub fn simulate(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let default_seconds = "2".to_string();
    let default_task = "cpu_intensive".to_string();
    
    let seconds_param = req.query.get("seconds").unwrap_or(&default_seconds);
    let task_param = req.query.get("task").unwrap_or(&default_task);
    
    let seconds = match seconds_param.parse::<u64>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'seconds' must be a valid positive integer"),
    };
    
    if seconds > 10 {
        return bad_request("Parameter 'seconds' must be <= 10");
    }
    
    let start = now_ms_since_epoch();
    
    match task_param.as_str() {
        "cpu_intensive" => {
            // Simular trabajo CPU-intensivo
            let mut result = 0u64;
            for i in 0..(seconds * 1_000_000) {
                result += i;
            }
            let elapsed = now_ms_since_epoch() - start;
            
            let body = format!(
                r#"{{"task":"{}","seconds":{},"result":{},"elapsed_ms":{}}}"#,
                task_param, seconds, result, elapsed
            );
            json_ok(body.into_bytes())
        },
        "io_intensive" => {
            // Simular trabajo IO-intensivo
            std::thread::sleep(std::time::Duration::from_secs(seconds));
            let elapsed = now_ms_since_epoch() - start;
            
            let body = format!(
                r#"{{"task":"{}","seconds":{},"elapsed_ms":{}}}"#,
                task_param, seconds, elapsed
            );
            json_ok(body.into_bytes())
        },
        _ => bad_request("Parameter 'task' must be 'cpu_intensive' or 'io_intensive'"),
    }
}

GET /loadtest?tasks=n&sleep=x
pub fn loadtest(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let default_tasks = "10".to_string();
    let default_sleep = "100".to_string();
    
    let tasks_param = req.query.get("tasks").unwrap_or(&default_tasks);
    let sleep_param = req.query.get("sleep").unwrap_or(&default_sleep);
    
    let tasks = match tasks_param.parse::<u32>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'tasks' must be a valid positive integer"),
    };
    
    let sleep_ms = match sleep_param.parse::<u64>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'sleep' must be a valid positive integer"),
    };
    
    if tasks == 0 || tasks > 100 {
        return bad_request("Parameter 'tasks' must be between 1 and 100");
    }
    
    if sleep_ms > 1000 {
        return bad_request("Parameter 'sleep' must be <= 1000 milliseconds");
    }
    
    let start = now_ms_since_epoch();
    let mut results = Vec::new();
    
    for _i in 0..tasks {
        let task_start = now_ms_since_epoch();
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
        let task_elapsed = now_ms_since_epoch() - task_start;
        results.push(task_elapsed);
    }
    
    let total_elapsed = now_ms_since_epoch() - start;
    let avg_elapsed = results.iter().sum::<u128>() / tasks as u128;
    
    let results_json = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    let body = format!(
        r#"{{"tasks":{},"sleep_ms":{},"total_elapsed_ms":{},"avg_task_elapsed_ms":{},"results":{}}}"#,
        tasks, sleep_ms, total_elapsed, avg_elapsed, results_json
    );
    
    json_ok(body.into_bytes())
}
 */

/// GET /createfile?name=filename&content=text&repeat=x
pub fn createfile(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let name_param = req.query.get("name");
    let content_param = req.query.get("content");
    let default_repeat = "1".to_string();
    let repeat_param = req.query.get("repeat").unwrap_or(&default_repeat);
    
    if name_param.is_none() || content_param.is_none() {
        return bad_request("Parameters 'name' and 'content' are required");
    }
    
    let filename = name_param.unwrap();
    let content = content_param.unwrap();
    
    let repeat = match repeat_param.parse::<u32>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'repeat' must be a valid positive integer"),
    };
    
    if filename.is_empty() {
        return bad_request("Parameter 'name' cannot be empty");
    }
    
    if repeat == 0 || repeat > 1000 {
        return bad_request("Parameter 'repeat' must be between 1 and 1000");
    }

    
    // Crear directorio data si no existe
    std::fs::create_dir_all("data").unwrap_or_default();
    
    // Construir contenido repetido
    let mut file_content = String::new();
    for _ in 0..repeat {
        file_content.push_str(content);
        file_content.push('\n');
    }
    
    // Escribir archivo
    let file_path = format!("data/{}", filename);


    if std::path::Path::new(&file_path).exists() {
    return (409, "application/json",
        format!(r#"{{"error":"conflict","message":"File exists","filename":"{}"}}"#, filename).into_bytes());
    }

    match std::fs::write(&file_path, file_content) {
        Ok(_) => {
            let file_size = std::fs::metadata(&file_path)
                .map(|m| m.len())
                .unwrap_or(0);
            
            let body = format!(
                r#"{{"filename":"{}","file_path":"{}","content_length":{},"repeat":{},"file_size_bytes":{}}}"#,
                filename, file_path, content.len(), repeat, file_size
            );
            json_ok(body.into_bytes())
        },
        Err(e) => {
            (
                500,
                "application/json",
                format!(
                    r#"{{"error":"file_error","message":"Failed to create file: {}","filename":"{}"}}"#,
                    e, filename
                ).into_bytes(),
            )
        }
    }
}

/// GET /deletefile?name=filename
pub fn deletefile(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let name_param = match req.query.get("name") {
        Some(n) => n,
        None => return bad_request("Parameter 'name' is required"),
    };

    if !validate_filename(name_param) {
        return bad_request("Invalid 'name': must be a plain filename (no path separators or '..').");
    }

    let file_path = format!("data/{}", name_param);

    // Si no existe → 409 (conflict) según enunciado
    if !std::path::Path::new(&file_path).exists() {
        return (
            409,
            "application/json",
            format!(r#"{{"error":"conflict","message":"File not found","filename":"{}","file_path":"{}"}}"#,
                    name_param, file_path).into_bytes(),
        );
    }

    match std::fs::remove_file(&file_path) {
        Ok(_) => {
            let body = format!(
                r#"{{"filename":"{}","file_path":"{}","message":"File deleted successfully"}} "#,
                name_param, file_path
            );
            json_ok(body.into_bytes())
        }
        Err(e) => {
            // Si por permisos o bloqueo falla → 500
            (
                500,
                "application/json",
                format!(r#"{{"error":"file_error","message":"Failed to delete file: {}","filename":"{}","file_path":"{}"}}"#,
                        e, name_param, file_path).into_bytes(),
            )
        }
    }

}
fn validate_filename(name: &str) -> bool {
    // Validar que el nombre no contenga separadores de ruta ni secuencias ".."
    !name.contains('/') && !name.contains('\\') && !name.contains("..") && !name.is_empty()
}   

/// Helper: determina a qué pool pertenece un comando (para métricas de colas/workers)
fn get_command_pool(cmd: &str) -> Option<&'static str> {
    match cmd {
        // Básicos → pool "basic"
        "status" | "timestamp" | "reverse" | "toupper" | "help" | "random" | "hash" |
        "createfile" | "deletefile" | "metrics" | "jobs/submit" | "jobs/status" |
        "jobs/result" | "jobs/cancel" => Some("basic"),
        // CPU-bound → pool "cpu"
        "fibonacci" | "isprime" | "factor" | "pi" | "mandelbrot" | "matrixmul" => Some("cpu"),
        // IO-bound → pool "io"
        "sleep" | "sortfile" | "wordcount" | "grep" | "compress" | "hashfile" => Some("io"),
        _ => None,
    }
}

/// GET /metrics
/// Retorna métricas por comando según el enunciado
pub fn metrics(state: &Shared, _req: &Request) -> (u16, &'static str, Vec<u8>) {
    use std::collections::HashMap;
    
    // Obtener métricas por comando
    let command_stats = state.metrics.get_all_command_stats();
    
    // Snapshots de colas por pool
    let qb = state.pools.basic.snapshot();
    let qc = state.pools.cpu.snapshot();
    let qi = state.pools.io.snapshot();
    
    // Mapear pools a información
    let pools_info: HashMap<&str, (usize, usize, usize)> = HashMap::from([
        ("basic", (qb.pending, qb.max_depth, qb.workers)),
        ("cpu", (qc.pending, qc.max_depth, qc.workers)),
        ("io", (qi.pending, qi.max_depth, qi.workers)),
    ]);
    
    // Construir JSON por comando según formato del enunciado
    let mut queues_json = String::new();
    let mut workers_json = String::new();
    let mut latency_json = String::new();
    
    let mut first_cmd = true;
    for (cmd, stats) in &command_stats {
        if !first_cmd {
            queues_json.push(',');
            workers_json.push(',');
            latency_json.push(',');
        }
        first_cmd = false;
        
        // Obtener información del pool para este comando
        let pool_name = get_command_pool(cmd).unwrap_or("basic");
        let (pending, _max_depth, total_workers) = pools_info.get(pool_name)
            .copied()
            .unwrap_or((0, 0, 0));
        
        // Workers ocupados: estim.init based on pending
        let busy_workers = if pending > 0 && pending < total_workers {
            pending
        } else if pending >= total_workers {
            total_workers
        } else {
            0
        };
        
        // Queues: pending por comando (compartimos la cola del pool)
        queues_json.push_str(&format!(r#""{}":{}"#, cmd, pending));
        
        // Workers: total y busy por comando
        workers_json.push_str(&format!(
            r#""{}":{{"total":{},"busy":{}}}"#,
            cmd, total_workers, busy_workers
        ));
        
        // Latency: p50, p95, p99 por comando (usando exec_ms como latencia principal)
        latency_json.push_str(&format!(
            r#""{}":{{"p50":{},"p95":{},"p99":{},"avg_wait_ms":{:.2},"avg_exec_ms":{:.2},"stddev_wait_ms":{:.2},"stddev_exec_ms":{:.2},"count":{}}}"#,
            cmd,
            stats.p50_exec_ms, stats.p95_exec_ms, stats.p99_exec_ms,
            stats.avg_wait_ms, stats.avg_exec_ms,
            stats.stddev_wait_ms, stats.stddev_exec_ms,
            stats.count
        ));
    }
    
    // Calcular throughput global
    let detailed_metrics = state.metrics.detailed_snapshot();
    let uptime_ms = now_ms_since_epoch() - state.started_ms;
    let requests_per_second = if uptime_ms > 0 { 
        (detailed_metrics.handled * 1000) / uptime_ms as u64 
    } else { 
        0 
    };
    
    // Formato según enunciado: { "queues": {...}, "workers": {...}, "latency_ms": {...} }
    let body = format!(
        r#"{{"queues":{{{}}},"workers":{{{}}},"latency_ms":{{{}}},"throughput":{{"requests_per_second":{}}},"requests":{{"accepted":{},"handled":{},"errors":{}}}}}"#,
        queues_json,
        workers_json,
        latency_json,
        requests_per_second,
        detailed_metrics.accepted,
        detailed_metrics.handled,
        detailed_metrics.errors
    );
    
    json_ok(body.into_bytes())
}

/// 404 por defecto
pub fn not_found(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    not_found_json(&req.path)
}

// ================= Algoritmos adicionales =================

/// O( n ) iterativo, limitado a n<=93 para evitar overflow en u64
pub fn fibonacci_calc(n: u32) -> Option<u64> {
    if n > 93 {
        return None;
    }
    if n == 0 {
        return Some(0);
    }
    if n == 1 {
        return Some(1);
    }
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 2..=n {
        let t = a + b;
        a = b;
        b = t;
    }
    Some(b)
}

/// GET /fibonacci?n=NUM
pub fn fibonacci(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let n_str = req.query.get("num").or_else(|| req.query.get("n")).map(String::as_str).unwrap_or("10");
    let n = match n_str.parse::<u32>() {
        Ok(v) => v,
        Err(_) => return bad_request("invalid_param: n must be u32"),
    };
    match fibonacci_calc(n) {
        Some(ans) => json_ok(
            format!(
                r#"{{"success":true,"n":{},"fibonacci":{},"message":"Fibonacci({}) = {}"}}"#,
                n, ans, n, ans
            )
            .into_bytes(),
        ),
        None => bad_request("n too large: overflow for u64 (n>93)"),
    }
}

/// primalidad simple por división hasta sqrt(n)
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
    let limit = (n as f64).sqrt() as u64;
    let mut i = 3;
    while i <= limit {
        if n % i == 0 {
            return false;
        }
        i += 2;
    }
    true
}

/// GET /isprime?n=NUM
pub fn isprime(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let n_str = req.query.get("n").map(String::as_str).unwrap_or("17");
    let n = match n_str.parse::<u64>() {
        Ok(v) => v,
        Err(_) => return bad_request("invalid_param: n must be u64"),
    };
    if n > 1_000_000 {
        return bad_request("n too large (max 1_000_000)");
    }
    let prime = is_prime_number(n);
    json_ok(
        format!(
            r#"{{"success":true,"n":{},"is_prime":{},"message":"{} is {}"}}"#,
            n,
            prime,
            n,
            if prime { "prime" } else { "not prime" }
        )
        .into_bytes(),
    )
}

/// GET /sleep?ms=X   (límite 5000ms)
pub fn sleep(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let seconds = req.query.get("seconds").and_then(|s| s.parse::<u64>().ok());
    let ms = match (seconds, req.query.get("ms")) {
    (Some(s), _) => s.saturating_mul(1000),
    (None, Some(ms_str)) => ms_str.parse::<u64>().unwrap_or(1000),
    _ => 1000
    };
    if ms > 15000 {
        return bad_request("ms too large (max 15000)");
    }
    std::thread::sleep(std::time::Duration::from_millis(ms));
    json_ok(
        format!(
            r#"{{"success":true,"slept_ms":{},"message":"Slept for {} milliseconds"}}"#,
            ms, ms
        )
        .into_bytes(),
    )
}

/// GET /help — JSON canónico (claves sin query; ejemplos dentro)
pub fn help(_state: &Shared, _req: &Request) -> (u16, &'static str, Vec<u8>) {
    let body = r#"{
  "endpoints": {
    "/status": {
      "description": "Server status information",
      "example": "/status"
    },
    "/timestamp": {
      "description": "Current server timestamp",
      "example": "/timestamp"
    },
    "/reverse": {
      "description": "Reverse a string",
      "parameters": { "text": "The string to reverse" },
      "example": "/reverse?text=hello"
    },
    "/toupper": {
      "description": "Uppercase a string",
      "parameters": { "text": "The string to convert" },
      "example": "/toupper?text=hello"
    },
    "/fibonacci": {
      "description": "Calculate the n-th Fibonacci number",
      "parameters": { "n": "0..93" },
      "example": "/fibonacci?n=10"
    },
    "/isprime": {
      "description": "Check if a number is prime",
      "parameters": { "n": "2..1_000_000" },
      "example": "/isprime?n=17"
    },
    "/sleep": {
      "description": "Sleep some milliseconds",
      "parameters": { "ms": "1..5000" },
      "example": "/sleep?ms=1000"
    },
    "/help": {
      "description": "This help message",
      "example": "/help"
    }
    "/random": {
    "description":"Generate random integers",
    "parameters":{"count":"1..1000","min":"int","max":"int"}
    },
    "/hash":   {
    "description":"SHA-256 of input text",
    "parameters":{"text":"string"}
    },
    "/simulate":{
    "description":"Toy workload",
    "parameters":{"seconds":"1..10",
    "task":"cpu_intensive|io_intensive"}
    },
    "/loadtest":{
    "description":"Run N sleep tasks",
    "parameters":{"tasks":"1..100","sleep":"ms"}
    },
    "/createfile":{
    "description":"Create file with repeated content",
    "parameters":{"name":"filename","content":"text","repeat":"1..1000"}
    },
    "/deletefile":{
    "description":"Delete file",
    "parameters":{"name":"filename"}
    },
    "/isprime":{
    "description":"Primality test",
    "parameters":{"n":"u64","method":"division|miller-rabin|auto"}
    },
    "/factor":{
    "description":"Prime factorization",
    "parameters":{"n":"u64<=1e6"}
    },
    "/pi":{
    "description":"Pi digits (spigot)",
    "parameters":{"digits":"1..1000"}
    },
    "/mandelbrot":{
    "description":"Mandelbrot iterations matrix",
    "parameters":{"width":"1..1000","height":"1..1000","max_iter":"1..10000"}
    },
    "/matrixmul":{
    "description":"Matrix multiply N x N, returns SHA-256",
    "parameters":{"size":"1..1000","seed":"u64"}
    },
    "/sortfile":{"description":"Sort integers file",
    "parameters":{"name":"file","algo":"merge|quick"}
    },
    "/wordcount":{
    "description":"Count lines, words, bytes",
    "parameters":{"name":"file"}
    },
    "/grep":{
    "description":"Count matches + first 10 lines",
    "parameters":{"name":"file","pattern":"string"}
    },
    "/compress":{
    "description":"Compress file",
    "parameters":{"name":"file","codec":"gzip|xz"}
    },
    "/hashfile":{
    "description":"SHA-256 of file",
    "parameters":{"name":"file","algo":"sha256"}}
  }
}"#;
    json_ok(body.as_bytes().to_vec())
}

/// --------- Wrappers "puros" usados por los tests (no requieren Shared/Request) ---------

pub fn handle_fibonacci(query_params: &HashMap<String, String>) -> String {
    let n_str = query_params.get("n").map(String::as_str).unwrap_or("10");
    let n = match n_str.parse::<u32>() {
        Ok(v) => v,
        Err(_) => {
            return format!(
                r#"{{"error":"invalid_parameter","message":"Parameter 'n' must be a valid positive integer","parameter":"{}"}}"#,
                n_str
            );
        }
    };

    match fibonacci_calc(n) {
        Some(ans) => format!(
            r#"{{"success":true,"n":{},"fibonacci":{},"message":"Fibonacci({}) = {}"}}"#,
            n, ans, n, ans
        ),
        None => r#"{"error":"calculation_error","message":"Cannot calculate Fibonacci for n > 93 (would cause overflow)","n":94}"#.to_string(),
    }
}

pub fn handle_isprime(query_params: &HashMap<String, String>) -> String {
    let n_str = query_params.get("n").map(String::as_str).unwrap_or("17");
    let n = match n_str.parse::<u64>() {
        Ok(v) => v,
        Err(_) => {
            return format!(
                r#"{{"error":"invalid_parameter","message":"Parameter 'n' must be a valid positive integer","parameter":"{}"}}"#,
                n_str
            );
        }
    };

    if n > 1_000_000 {
        return format!(
            r#"{{"error":"parameter_too_large","message":"Parameter 'n' must be <= 1,000,000 for performance reasons","n":{}}}"#,
            n
        );
    }

    let prime = is_prime_number(n);
    format!(
        r#"{{"success":true,"n":{},"is_prime":{},"message":"{} is {}"}}"#,
        n,
        if prime { "true" } else { "false" },
        n,
        if prime { "prime" } else { "not prime" }
    )
}

pub fn handle_sleep(query_params: &HashMap<String, String>) -> String {
    let ms_str = query_params.get("ms").map(String::as_str).unwrap_or("1000");
    let ms = match ms_str.parse::<u64>() {
        Ok(v) => v,
        Err(_) => {
            return format!(
                r#"{{"error":"invalid_parameter","message":"Parameter 'ms' must be a valid positive integer","parameter":"{}"}}"#,
                ms_str
            );
        }
    };

    if ms > 5000 {
        return format!(
            r#"{{"error":"parameter_too_large","message":"Parameter 'ms' must be <= 5000 milliseconds","ms":{}}}"#,
            ms
        );
    }

    std::thread::sleep(std::time::Duration::from_millis(ms));
    format!(
        r#"{{"success":true,"slept_ms":{},"message":"Slept for {} milliseconds"}}"#,
        ms, ms
    )
}

pub fn handle_help() -> String {
    r#"{
  "endpoints": {
    "/status": {
      "description": "Server status information",
      "example": "/status"
    },
    "/timestamp": {
      "description": "Current server timestamp",
      "example": "/timestamp"
    },
    "/reverse": {
      "description": "Reverse a string",
      "parameters": { "text": "The string to reverse" },
      "example": "/reverse?text=hello"
    },
    "/toupper": {
      "description": "Uppercase a string",
      "parameters": { "text": "The string to convert" },
      "example": "/toupper?text=hello"
    },
    "/fibonacci": {
      "description": "Calculate the n-th Fibonacci number",
      "parameters": { "n": "0..93" },
      "example": "/fibonacci?n=10"
    },
    "/isprime": {
      "description": "Check if a number is prime",
      "parameters": { "n": "2..1_000_000" },
      "example": "/isprime?n=17"
    },
    "/sleep": {
      "description": "Sleep some milliseconds",
      "parameters": { "ms": "1..5000" },
      "example": "/sleep?ms=1000"
    },
    "/help": {
      "description": "This help message",
      "example": "/help"
    }
  }
}"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        // Handlers (renombrados para claridad)
        status as status_handler,
        timestamp as timestamp_handler,
        reverse as reverse_handler,
        toupper as toupper_handler,
        handle_fibonacci,
        handle_isprime,
        handle_sleep,
        handle_help,
        // función pura
        fibonacci_calc as fib,
        is_prime_number,
    };

    // 👇 IMPORTANTE: estas dos importaciones **dentro** del módulo de pruebas
    use crate::router::Router;
    use crate::workers::Pools;

    use crate::config::Config;
    use crate::core::{now_ms_since_epoch, AppState, Request};
    use crate::metrics::Metrics;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn fake_state() -> Arc<AppState> {
        let cfg = Config::from_env_or_default();
        let router = Router::new();
        let pools = Arc::new(Pools::new_dummy()); // pools “dummy” para tests

        Arc::new(AppState {
            cfg,
            router,
            started_ms: now_ms_since_epoch(),
            metrics: Arc::new(Metrics::default()),
            pools, // <- campo nuevo obligatorio
            job_store: Arc::new(crate::jobs::JobStore::new()),
        })
    }

    fn req_from(path: &str) -> Request {
        let (path_only, query) = if let Some((p, q)) = path.split_once('?') {
            (p.to_string(), parse_query(q))
        } else {
            (path.to_string(), HashMap::new())
        };

        Request {
            method: "GET".into(),
            path: path_only,
            query,
            http_version: "HTTP/1.0".into(),
            request_id: "test-req".into(),
        }
    }

    fn parse_query(qs: &str) -> HashMap<String, String> {
        let mut m = HashMap::new();
        for pair in qs.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                m.insert(k.to_string(), v.to_string());
            }
        }
        m
    }

    // ------------------- pruebas existentes -------------------

    #[test]
    fn status_returns_ok_json() {
        let state = fake_state();
        let req = req_from("/status");
        let (code, ctype, body) = status_handler(&state, &req);
        assert_eq!(code, 200);
        assert_eq!(ctype, "application/json");
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"status\":\"ok\""));
    }

    #[test]
    fn reverse_ok() {
        let state = fake_state();
        let req = req_from("/reverse?text=hola");
        let (code, ctype, body) = reverse_handler(&state, &req);
        assert_eq!(code, 200);
        assert_eq!(ctype, "application/json");
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"input\":\"hola\""));
        assert!(s.contains("\"output\":\"aloh\""));
    }

    #[test]
    fn reverse_missing_param_is_400() {
        let state = fake_state();
        let req = req_from("/reverse");
        let (code, _ctype, body) = reverse_handler(&state, &req);
        assert_eq!(code, 400);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("missing_param"));
    }

    #[test]
    fn toupper_ok() {
        let state = fake_state();
        let req = req_from("/toupper?text=AbCd");
        let (code, _ctype, body) = toupper_handler(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"output\":\"ABCD\""));
    }

    // ------------------- pruebas nuevas -------------------

    #[test]
    fn timestamp_is_json_with_unix_ms() {
        let state = fake_state();
        let req = req_from("/timestamp");
        let (code, ctype, body) = timestamp_handler(&state, &req);
        assert_eq!(code, 200);
        assert_eq!(ctype, "application/json");
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("unix_ms"));
    }

    #[test]
    fn fibonacci_ok_10() {
        assert_eq!(fib(10), Some(55));
    }

    #[test]
    fn fibonacci_rejects_overflow() {
        assert_eq!(fib(94), None);
    }

    #[test]
    fn handle_fibonacci_parsing_and_ok() {
        let mut q = HashMap::new();
        q.insert("n".into(), "8".into());
        let out = handle_fibonacci(&q);
        assert!(out.contains("\"fibonacci\":21"));
    }

    #[test]
    fn handle_fibonacci_invalid_param() {
        let mut q = HashMap::new();
        q.insert("n".into(), "hola".into());
        let out = handle_fibonacci(&q);
        assert!(out.contains("\"invalid_parameter\""));
    }

    #[test]
    fn isprime_basic() {
        assert!(is_prime_number(97));
        assert!(!is_prime_number(1));
        assert!(!is_prime_number(100));
    }

    #[test]
    fn handle_isprime_ok() {
        let mut q = HashMap::new();
        q.insert("n".into(), "97".into());
        let out = handle_isprime(&q);
        assert!(out.contains("\"is_prime\":true"));
    }

    #[test]
    fn handle_sleep_caps_to_5s() {
        let mut q = HashMap::new();
        q.insert("ms".into(), "6000".into());
        let out = handle_sleep(&q);
        assert!(out.contains("\"parameter_too_large\""));
    }

    #[test]
    fn not_found_has_path() {
        let state = fake_state();
        let req = req_from("/noexiste");
        let (code, _ctype, body) = super::not_found(&state, &req);
        assert_eq!(code, 404);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"not_found\""));
        assert!(s.contains("/noexiste"));
    }

    #[test]
    fn random_invalid_count_zero() {
        let state = fake_state();
        let req = req_from("/random?count=0&min=1&max=10");
        let (code, _ctype, _body) = super::random(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn random_min_ge_max() {
        let state = fake_state();
        let req = req_from("/random?count=5&min=10&max=10");
        let (code, _ctype, _body) = super::random(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn random_large_count_ok() {
        let state = fake_state();
        let req = req_from("/random?count=1000&min=1&max=100");
        let (code, _ctype, body) = super::random(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"count\":1000"));
        assert!(s.contains("\"numbers\""));
    }

    // --------- DELETEFILE ---------

    #[test]
    fn deletefile_not_found_returns_404() {
        let state = fake_state();
        let req = req_from("/deletefile?name=__no_exist__.txt");
        let (code, _ctype, body) = super::deletefile(&state, &req);
        // Nuevo comportamiento: 409 Conflict cuando no existe
        assert_eq!(code, 409);
        // Guardar salida en txt
        std::fs::create_dir_all("data").unwrap_or_default();
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("data/test_outputs.txt")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "[deletefile_missing] code={} body={}", code, std::str::from_utf8(&body).unwrap_or("<utf8_err>"))
            });
    }

    #[test]
    fn deletefile_invalid_traversal_returns_400() {
        // Requiere que hayas agregado validación para rechazar '..' o separadores.
        let state = fake_state();
        let req = req_from("/deletefile?name=../evil.txt");
        let (code, _ctype, body) = super::deletefile(&state, &req);
        assert_eq!(code, 400);
        // Log a txt
        std::fs::create_dir_all("data").unwrap_or_default();
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("data/test_outputs.txt")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "[deletefile_traversal] code={} body={}", code, std::str::from_utf8(&body).unwrap_or("<utf8_err>"))
            });
    }

    #[test]
    fn deletefile_invalid_subdir_returns_400() {
        // Si decides permitir sólo archivos en data/ (sin subdirectorios), este test valida eso.
        let state = fake_state();
        let req = req_from("/deletefile?name=sub/nums.txt");
        let (code, _ctype, body) = super::deletefile(&state, &req);
        assert_eq!(code, 400);
        // Log a txt
        std::fs::create_dir_all("data").unwrap_or_default();
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("data/test_outputs.txt")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "[deletefile_subdir] code={} body={}", code, std::str::from_utf8(&body).unwrap_or("<utf8_err>"))
            });
    }

    #[test]
    fn createfile_conflict_returns_409_and_logs() {
        // Crear archivo inicial
        std::fs::create_dir_all("data").unwrap_or_default();
        let _ = std::fs::write("data/conflict.txt", "x");

        let state = fake_state();
        let req = req_from("/createfile?name=conflict.txt&content=abc&repeat=1");
        let (code, _ctype, body) = super::createfile(&state, &req);
        assert_eq!(code, 409);

        // Log a txt
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("data/test_outputs.txt")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "[createfile_conflict] code={} body={}", code, std::str::from_utf8(&body).unwrap_or("<utf8_err>"))
            });
    }

    #[test]
    fn help_json_has_canonical_keys() {
        let out = handle_help();

        // Chequeos mínimos (sin parser JSON externo)
        assert!(out.trim_start().starts_with('{'));
        assert!(out.contains("\"endpoints\""));

        for key in [
            "\"/status\"",
            "\"/timestamp\"",
            "\"/reverse\"",
            "\"/toupper\"",
            "\"/fibonacci\"",
            "\"/isprime\"",
            "\"/sleep\"",
            "\"/help\"",
        ] {
            assert!(out.contains(key), "missing endpoint key: {key}");
        }

        assert!(out.contains("/reverse?text=hello"));
        assert!(out.contains("/toupper?text=hello"));
        assert!(out.contains("/fibonacci?n=10"));
        assert!(out.contains("/isprime?n=17"));
        assert!(out.contains("/sleep?ms=1000"));
    }
}