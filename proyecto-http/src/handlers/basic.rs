//! Handlers básicos: /status, /timestamp, /reverse, /toupper,
//! y básicos extra: /fibonacci, /isprime, /sleep, /help, + 404.
//!
//! Todos los handlers siguen la misma firma:
//!   (state: &Shared, req: &Request) -> (status, content_type, body_bytes)

use crate::core::{now_ms_since_epoch, Request, Shared};
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
fn not_found_json(path: &str) -> (u16, &'static str, Vec<u8>) {
    (
        404,
        "application/json",
        format!(r#"{{"error":"not_found","path":"{}"}}"#, path).into_bytes(),
    )
}

/// GET /status
pub fn status(state: &Shared, _req: &Request) -> (u16, &'static str, Vec<u8>) {
    let (accepted, handled) = state.metrics.snapshot();

    // NUEVO: snapshots de colas
    let qb = state.pools.basic.snapshot();
    let qc = state.pools.cpu.snapshot();
    let qi = state.pools.io.snapshot();

    let body = format!(
        r#"{{
  "status":"ok",
  "port":{port},
  "pid":{pid},
  "uptime_ms":{uptime},
  "metrics":{{"accepted":{acc},"handled":{hdl}}},
  "queues":[
    {{"name":"{qb_name}","pending":{qb_pending},"max_depth":{qb_max},"workers":{qb_workers}}},
    {{"name":"{qc_name}","pending":{qc_pending},"max_depth":{qc_max},"workers":{qc_workers}}},
    {{"name":"{qi_name}","pending":{qi_pending},"max_depth":{qi_max},"workers":{qi_workers}}}
  ]
}}"#,
        port = state.cfg.port,
        pid = std::process::id(),
        uptime = now_ms_since_epoch().saturating_sub(state.started_ms),
        acc = accepted,
        hdl = handled,

        qb_name = qb.name, qb_pending = qb.pending, qb_max = qb.max_depth, qb_workers = qb.workers,
        qc_name = qc.name, qc_pending = qc.pending, qc_max = qc.max_depth, qc_workers = qc.workers,
        qi_name = qi.name, qi_pending = qi.pending, qi_max = qi.max_depth, qi_workers = qi.workers,
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
    let n_str = req.query.get("n").map(String::as_str).unwrap_or("10");
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
    let ms_str = req.query.get("ms").map(String::as_str).unwrap_or("1000");
    let ms = match ms_str.parse::<u64>() {
        Ok(v) => v,
        Err(_) => return bad_request("invalid_param: ms must be u64"),
    };
    if ms > 5000 {
        return bad_request("ms too large (max 5000)");
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
