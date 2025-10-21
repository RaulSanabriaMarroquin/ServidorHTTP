//! Handlers básicos: /status, /timestamp, /reverse, /toupper,
//! y básicos extra: /fibonacci, /isprime, /sleep, /help, + 404.
//!
//! Todos los handlers siguen la misma firma:
//!   (state: &Shared, req: &Request) -> (status, content_type, body_bytes)

use crate::core::{now_ms_since_epoch, Request, Shared};

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
    let body = format!(
        r#"{{
  "status":"ok",
  "port":{},
  "pid":{},
  "uptime_ms":{},
  "metrics":{{"accepted":{},"handled":{}}}
}}"#,
        state.cfg.port,
        std::process::id(),
        now_ms_since_epoch().saturating_sub(state.started_ms),
        accepted,
        handled
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

/// GET /help
pub fn help(_state: &Shared, _req: &Request) -> (u16, &'static str, Vec<u8>) {
    let body = r#"{
  "endpoints": {
    "/status": "Server status information",
    "/timestamp": "Current server timestamp",
    "/reverse?text=hello": "Reverse a string",
    "/toupper?text=hello": "Uppercase a string",
    "/fibonacci?n=10": "Calculate the n-th Fibonacci number (n<=93)",
    "/isprime?n=17": "Check if a number is prime (n<=1_000_000)",
    "/sleep?ms=1000": "Sleep some milliseconds (<=5000)",
    "/help": "This help message"
  }
}"#;
    json_ok(body.as_bytes().to_vec())
}
