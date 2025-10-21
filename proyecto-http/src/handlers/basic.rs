//! Handlers básicos: /status, /timestamp, /reverse, /toupper, y 404.

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
        Some(s) => json_ok(format!(r#"{{"input":"{}","output":"{}"}}"#, s, s.to_uppercase()).into_bytes()),
        None => bad_request("missing_param: text"),
    }
}

/// 404 por defecto
pub fn not_found(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    not_found_json(&req.path)
}
