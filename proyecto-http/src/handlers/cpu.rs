//! Handlers CPU-bound: /isprime, /factor, /pi, /mandelbrot, /matrixmul
//!
//! Todos los handlers siguen la misma firma:
//!   (state: &Shared, req: &Request) -> (status, content_type, body_bytes)

use crate::core::{now_ms_since_epoch, Request, Shared};
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::{Zero,One, Signed}; // (y los que ya uses: Zero, ToPrimitive, Signed, etc.)
use sha2::{Sha256, Digest};


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

/// GET /isprime?n=NUM&method=division|miller-rabin
pub fn isprime(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let start = now_ms_since_epoch();
    
    let n_param = req.query.get("n");
    let default_method = "auto".to_string();
    let method_param = req.query.get("method").unwrap_or(&default_method);
    
    if n_param.is_none() {
        return bad_request("Parameter 'n' is required");
    }
  
    let n = match n_param.unwrap().parse::<u64>() {
        Ok(val) => val,
        Err(_) => {
            return bad_request("Parameter 'n' must be a valid positive integer");
        }
    };
    
    if n > 10_000_000 {
        return bad_request("Parameter 'n' must be <= 10,000,000 for performance reasons");
    }

    
    // Determinar método automáticamente o usar el especificado
    let (is_prime, method_used) = match method_param.as_str() {
        "division" => (is_prime_number(n), "division"),
        "miller-rabin" => (is_prime_miller_rabin(n, 10), "miller-rabin"),
        "auto" => {
            if n < 10000 {
                (is_prime_number(n), "division")
            } else {
                (is_prime_miller_rabin(n, 10), "miller-rabin")
            }
        },
        _ => return bad_request("Parameter 'method' must be 'division', 'miller-rabin', or 'auto'"),
    };
    
    let elapsed = now_ms_since_epoch() - start;
    
    let body = format!(
        r#"{{"n":{},"is_prime":{},"method":"{}","elapsed_ms":{}}}"#,
        n, is_prime, method_used, elapsed
    );
    
    json_ok(body.into_bytes())
}

/// GET /factor?n=NUM
pub fn factor(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let start = now_ms_since_epoch();
    
    let n_param = req.query.get("n");
    if n_param.is_none() {
        return bad_request("Parameter 'n' is required");
    }
    
    let n = match n_param.unwrap().parse::<u64>() {
        Ok(val) => val,
        Err(_) => {
            return bad_request("Parameter 'n' must be a valid positive integer");
        }
    };
    
    if n == 0 || n == 1 {
        let elapsed = now_ms_since_epoch() - start;
        let body = format!(
            r#"{{"n":{},"factors":[],"elapsed_ms":{}}}"#,
            n, elapsed
        );
        return json_ok(body.into_bytes());
    }
    
    if n > 1_000_000 {
        return bad_request("Parameter 'n' must be <= 1,000,000 for performance reasons");
    }
    
    let factors = factorize(n);
    let elapsed = now_ms_since_epoch() - start;
    
    let factors_json = serde_json::to_string(&factors).unwrap_or_else(|_| "[]".to_string());
    let body = format!(
        r#"{{"n":{},"factors":{},"elapsed_ms":{}}}"#,
        n, factors_json, elapsed
    );
    
    json_ok(body.into_bytes())
}

// Cálculo de π con Chudnovsky (iterativo)
// - digits: número de dígitos decimales a devolver (capamos a 2000 por seguridad).
// - max_iters: máximo de iteraciones de la serie (cada iter da ~14 dígitos nuevos).
/// Cálculo de π con Chudnovsky + Binary Splitting, en puro Rust (num-bigint).
/// Retorna una cadena con `digits` decimales (p.ej. digits=10 → "3.1415926535").
fn calculate_pi_chudnovsky(digits: u32) -> String {
    if digits == 0 {
        return "3".to_string();
    }

    // Decimales extra para redondeo seguro en la división final
    let extra: u32 = 10;
    let prec: u32 = digits + extra;

    // Número de términos necesarios (≈ 14.181647462 decimales por término)
    let terms = ((prec as f64) / 14.181647462).ceil() as usize;

    // Binary splitting: computa (P,Q,T) para k in [0, terms)
    let (p, q, t) = bs_chudnovsky(0, terms);

    // Necesitamos 426880 * sqrt(10005) * Q / T
    // Escalamos sqrt para trabajar en enteros:
    // sqrt_scaled ≈ sqrt(10005 * 10^(2*prec)) = isqrt(10005) * 10^prec
    let ten = BigUint::from(10u32);
    let scale = ten.pow(prec);
    let sqrt_arg = BigUint::from(10005u32) * &scale * &scale;
    let sqrt_scaled = isqrt(&sqrt_arg);

    // 426880 * sqrt(10005) * Q
    let k426880 = BigUint::from(426880u32);
    let numer = k426880 * sqrt_scaled * q; // BigUint

    // Dividir por T (T puede ser negativo; tomamos valor absoluto y ajustamos signo)
    let t_abs = t.abs().to_biguint().unwrap();
    let mut pi_scaled = &numer / &t_abs; // truncado

    // Convertir a string y formatear con punto decimal
    // pi_scaled ~ floor( 10^prec * pi )
    let mut s = pi_scaled.to_string();
    if s.len() <= prec as usize {
        // anteponer ceros si hace falta
        let mut z = String::from("0".repeat(prec as usize + 1 - s.len()));
        z.push_str(&s);
        s = z;
    }
    // Insertar punto después del primer dígito
    let int_part = &s[..1];
    let frac_part_full = &s[1..];

    // Recortar a `digits` decimales (quitando los `extra`)
    let wanted = digits as usize;
    let frac_trimmed = if frac_part_full.len() >= wanted {
        &frac_part_full[..wanted]
    } else {
        frac_part_full
    };

    format!("{int_part}.{frac_trimmed}")
}

/// Binary splitting para Chudnovsky.
/// Devuelve (P,Q,T) como BigUint/BigInt para el rango [a,b).
fn bs_chudnovsky(a: usize, b: usize) -> (BigUint, BigUint, BigInt) {
    // Constantes de Chudnovsky
    const A: i64 = 13_591_409;
    const B: i64 = 545_140_134;
    // (640320^3)/24 = 10_939_058_860_032_000  (cabe en u64)
    const C3_24: u64 = 10_939_058_860_032_000;

    if b - a == 1 {
        // Caso base k = a
        if a == 0 {
            // P=Q=1, T=A
            return (BigUint::one(), BigUint::one(), BigInt::from(A));
        }

        // P(a) = (6a-5)(2a-1)(6a-1)
        let a_u = a as u64;
        let p = BigUint::from(6 * a_u - 5)
            * BigUint::from(2 * a_u - 1)
            * BigUint::from(6 * a_u - 1);

        // Q(a) = a^3 * C3_24
        let q = BigUint::from(a_u.pow(3)) * BigUint::from(C3_24);

        // T(a) = P(a) * (A + B a) * (-1)^a
        let ab = A as i128 + (B as i128) * (a as i128);
        // from_biguint necesita el enum Sign de num_bigint
        let mut t = BigInt::from_biguint(Sign::Plus, p.clone()) * BigInt::from(ab);
        if a % 2 == 1 {
            t = -t;
        }
        (p, q, t)
    } else {
        let m = (a + b) / 2;
        let (p1, q1, t1) = bs_chudnovsky(a, m);
        let (p2, q2, t2) = bs_chudnovsky(m, b);

        let p = &p1 * &p2;
        let q = &q1 * &q2;

        // t = t1*q2 + p1*t2  (con tipos BigInt/BigUint correctos)
        let t = t1 * BigInt::from_biguint(Sign::Plus, q2.clone())
            + BigInt::from_biguint(Sign::Plus, p1) * t2;

        (p, q, t)
    }
}


/// Entero sqrt: floor(sqrt(n)) para BigUint
fn isqrt(n: &BigUint) -> BigUint {
    if n.is_zero() {
        return BigUint::zero();
    }
    // Aproximación inicial: 1 << ((bits+1)/2)
    let mut x0 = BigUint::one() << ((n.bits() + 1) / 2);
    loop {
        let x1 = (&x0 + (n / &x0)) >> 1;
        if x1 >= x0 {
            return x0;
        }
        x0 = x1;
    }
}

/// --- Helpers para PI (Machin) ---

fn arctan_series(x: f64, terms: usize) -> f64 {
    // Serie de Taylor: arctan(x) = Σ (-1)^k * x^(2k+1)/(2k+1)
    let mut sum = 0.0f64;
    let mut sign = 1.0f64;
    let mut x_pow = x; // x^(2k+1) empieza en x^1
    for k in 0..terms {
        let denom = (2 * k + 1) as f64;
        sum += sign * x_pow / denom;
        sign = -sign;
        // siguiente potencia: multiplicar por x^2
        x_pow *= x * x;
    }
    sum
}

fn pi_with_machin(digits: u32) -> String {
    //  Machin: pi = 16*arctan(1/5) - 4*arctan(1/239)
    //  Elegimos #terms suficientemente grande para cubrir 'digits'
    //  Regla empírica simple: terms = digits + 10
    let terms = (digits as usize) + 10;
    let a = arctan_series(1.0/5.0,   terms);
    let b = arctan_series(1.0/239.0, terms);
    let pi = 16.0*a - 4.0*b;

    // Formatear con exactamente `digits` decimales
    if digits == 0 {
        return "3".to_string();
    }
    format!("{:.1$}", pi, digits as usize) // imprime 3.<digits>
}

/// GET /pi?digits=D
pub fn pi(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let start = now_ms_since_epoch();

    let digits_param = req.query.get("digits");
    if digits_param.is_none() {
        return bad_request("Parameter 'digits' is required");
    }
    let digits = match digits_param.unwrap().parse::<u32>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'digits' must be a valid positive integer"),
    };

    if digits == 0 || digits > 1000 {
        return bad_request("Parameter 'digits' must be between 1 and 1000");
    }

    let s = pi_with_machin(digits);
    let elapsed = now_ms_since_epoch() - start;
    let body = format!(r#"{{"digits":{},"pi":"{}","method":"machin","elapsed_ms":{}}}"#, digits, s, elapsed);
    (200, "application/json", body.into_bytes())
}

/// GET /mandelbrot?width=W&height=H&max_iter=I
pub fn mandelbrot(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let start = now_ms_since_epoch();
    
    let width_param = req.query.get("width");
    let height_param = req.query.get("height");
    let max_iter_param = req.query.get("max_iter");
    
    if width_param.is_none() || height_param.is_none() || max_iter_param.is_none() {
        return bad_request("Parameters 'width', 'height', and 'max_iter' are required");
    }
    
    let width = match width_param.unwrap().parse::<u32>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'width' must be a valid positive integer"),
    };
    
    let height = match height_param.unwrap().parse::<u32>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'height' must be a valid positive integer"),
    };
    
    let max_iter = match max_iter_param.unwrap().parse::<u32>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'max_iter' must be a valid positive integer"),
    };
    
    if width == 0 || width > 1000 || height == 0 || height > 1000 {
        return bad_request("Parameters 'width' and 'height' must be between 1 and 1000");
    }
    
    if max_iter == 0 || max_iter > 10000 {
        return bad_request("Parameter 'max_iter' must be between 1 and 10000");
    }
    
    let iterations = calculate_mandelbrot(width, height, max_iter);
    let elapsed = now_ms_since_epoch() - start;
    
    let iterations_json = serde_json::to_string(&iterations).unwrap_or_else(|_| "[]".to_string());
    let body = format!(
        r#"{{"width":{},"height":{},"max_iter":{},"iterations":{},"elapsed_ms":{}}}"#,
        width, height, max_iter, iterations_json, elapsed
    );
    
    json_ok(body.into_bytes())
}

/// GET /matrixmul?size=N&seed=S
pub fn matrixmul(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    use sha2::{Sha256, Digest};
    use hex::encode as hex_encode;

    let start = now_ms_since_epoch();

    let size_param = req.query.get("size");
    let default_seed = "123".to_string();
    let seed_param = req.query.get("seed").unwrap_or(&default_seed);

    if size_param.is_none() {
        return bad_request("Parameter 'size' is required");
    }

    let size = match size_param.unwrap().parse::<usize>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'size' must be a valid positive integer"),
    };

    let seed = match seed_param.parse::<u64>() {
        Ok(val) => val,
        Err(_) => return bad_request("Parameter 'seed' must be a valid integer"),
    };

    if size == 0 || size > 1000 {
        return bad_request("Parameter 'size' must be between 1 and 1000");
    }

    // Multiplica y hashea con SHA-256 sobre los bytes de f64 (determinístico)
    let result_hash = multiply_matrices_sha256(size, seed);
    let elapsed = now_ms_since_epoch() - start;

    let body = format!(
        r#"{{"size":{},"seed":{},"result_sha256":"{}","elapsed_ms":{}}}"#,
        size, seed, result_hash, elapsed
    );
    json_ok(body.into_bytes())
}

// Funciones auxiliares

/// Verifica si un número es primo usando división hasta √n
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
    
    let sqrt_n = (n as f64).sqrt() as u64;
    for i in (3..=sqrt_n).step_by(2) {
        if n % i == 0 {
            return false;
        }
    }
    
    true
}

/// Verifica si un número es primo usando el algoritmo de Miller-Rabin
/// Más eficiente para números grandes
fn is_prime_miller_rabin(n: u64, k: usize) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    
    // Escribir n-1 como d * 2^r
    let mut d = n - 1;
    let mut r = 0;
    while d % 2 == 0 {
        d /= 2;
        r += 1;
    }
    
    // Realizar k rondas del test
    for _ in 0..k {
        let a = 2 + (rand_u64() % (n - 4));
        let mut x = mod_pow(a, d, n);
        
        if x == 1 || x == n - 1 {
            continue;
        }
        
        let mut found = false;
        for _ in 0..r - 1 {
            x = mod_pow(x, 2, n);
            if x == n - 1 {
                found = true;
                break;
            }
        }
        
        if !found {
            return false;
        }
    }
    
    true
}

/// Generador de números aleatorios simple para Miller-Rabin
fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    now.as_nanos() as u64
}

/// Exponenciación modular: (base^exp) % mod
fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }
    
    let mut result = 1;
    base %= modulus;
    
    while exp > 0 {
        if exp % 2 == 1 {
            result = (result * base) % modulus;
        }
        exp >>= 1;
        base = (base * base) % modulus;
    }
    
    result
}

/// Factoriza un número en primos
fn factorize(n: u64) -> Vec<Vec<u64>> {
    let mut factors = Vec::new();
    let mut num = n;
    
    // Factorizar por 2
    if num % 2 == 0 {
        let mut count = 0;
        while num % 2 == 0 {
            num /= 2;
            count += 1;
        }
        factors.push(vec![2, count]);
    }
    
    // Factorizar por números impares
    let mut i = 3;
    while i * i <= num {
        if num % i == 0 {
            let mut count = 0;
            while num % i == 0 {
                num /= i;
                count += 1;
            }
            factors.push(vec![i, count]);
        }
        i += 2;
    }
    
    // Si queda algo, es un primo
    if num > 1 {
        factors.push(vec![num, 1]);
    }
    
    factors
}


/// Calcula el conjunto de Mandelbrot
fn calculate_mandelbrot(width: u32, height: u32, max_iter: u32) -> Vec<Vec<u32>> {
    let mut result = vec![vec![0; width as usize]; height as usize];
    
    for y in 0..height {
        for x in 0..width {
            let cx = -2.0 + (x as f64) * 3.0 / (width as f64);
            let cy = -1.0 + (y as f64) * 2.0 / (height as f64);
            
            let mut zx = 0.0;
            let mut zy = 0.0;
            let mut iter = 0;
            
            while zx * zx + zy * zy < 4.0 && iter < max_iter {
                let tmp = zx * zx - zy * zy + cx;
                zy = 2.0 * zx * zy + cy;
                zx = tmp;
                iter += 1;
            }
            
            result[y as usize][x as usize] = iter;
        }
    }
    
    result
}

/// Multiplica dos matrices N x N determinísticas y devuelve SHA-256 (hex)
fn multiply_matrices_sha256(size: usize, seed: u64) -> String {
    // Generar matrices determinísticas (como ya hacías)
    let mut a = vec![vec![0.0; size]; size];
    let mut b = vec![vec![0.0; size]; size];
    for i in 0..size {
        for j in 0..size {
            a[i][j] = pseudo_random(seed + (i * size + j) as u64);
            b[i][j] = pseudo_random(seed + (i * size + j) as u64 + 1000);
        }
    }

    let mut c = vec![vec![0.0; size]; size];
    for i in 0..size {
        for j in 0..size {
            let mut acc = 0.0;
            for k in 0..size {
                acc += a[i][k] * b[k][j];
            }
            c[i][j] = acc;
        }
    }

    // SHA-256 del resultado (en binario) y devolver hex (64 chars)
    let mut hasher = Sha256::new();
    for row in &c {
        for &v in row {
            hasher.update(v.to_le_bytes());
        }
    }
    let digest = hasher.finalize();
    hex::encode(digest)
}


/// Generador pseudoaleatorio simple
fn pseudo_random(seed: u64) -> f64 {
    let mut x = seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    (x as f64) / (u64::MAX as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{now_ms_since_epoch, AppState, Request};
    use crate::config::Config;
    use crate::router::Router;
    use crate::workers::Pools;
    use crate::metrics::Metrics;
    use crate::jobs::JobStore;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn fake_state() -> Arc<AppState> {
        let cfg = Config::from_env_or_default();
        let router = Router::new();
        let pools = Arc::new(Pools::new_dummy());

        Arc::new(AppState {
            cfg,
            router,
            started_ms: now_ms_since_epoch(),
            metrics: Arc::new(Metrics::default()),
            pools,
            job_store: Arc::new(JobStore::new()),
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

    #[test]
    fn test_isprime_handler() {
        let state = fake_state();
        let req = req_from("/isprime?n=97");
        let (code, _ctype, body) = isprime(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"is_prime\":true"));
    }

    #[test]
    fn test_isprime_not_prime() {
        let state = fake_state();
        let req = req_from("/isprime?n=100");
        let (code, _ctype, body) = isprime(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"is_prime\":false"));
    }

    #[test]
    fn test_isprime_missing_param() {
        let state = fake_state();
        let req = req_from("/isprime");
        let (code, _ctype, _body) = isprime(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_isprime_invalid_param() {
        let state = fake_state();
        let req = req_from("/isprime?n=abc");
        let (code, _ctype, _body) = isprime(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_isprime_too_large() {
        let state = fake_state();
        let req = req_from("/isprime?n=2000000");
        let (code, _ctype, _body) = isprime(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_factor_handler() {
        let state = fake_state();
        let req = req_from("/factor?n=360");
        let (code, _ctype, body) = factor(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"factors\""));
    }

    #[test]
    fn test_factor_prime() {
        let state = fake_state();
        let req = req_from("/factor?n=97");
        let (code, _ctype, body) = factor(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"factors\":[[97,1]]"));
    }

    #[test]
    fn test_factor_zero() {
        let state = fake_state();
        let req = req_from("/factor?n=0");
        let (code, _ctype, body) = factor(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"factors\":[]"));
    }

    #[test]
    fn test_pi_handler() {
        let state = fake_state();
        let req = req_from("/pi?digits=10");
        let (code, _ctype, body) = pi(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"pi\":\"3.141592653\""));
    }

    #[test]
    fn test_pi_missing_param() {
        let state = fake_state();
        let req = req_from("/pi");
        let (code, _ctype, _body) = pi(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_pi_invalid_param() {
        let state = fake_state();
        let req = req_from("/pi?digits=abc");
        let (code, _ctype, _body) = pi(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_pi_too_large() {
        let state = fake_state();
        let req = req_from("/pi?digits=2000");
        let (code, _ctype, _body) = pi(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_mandelbrot_handler() {
        let state = fake_state();
        let req = req_from("/mandelbrot?width=10&height=10&max_iter=100");
        let (code, _ctype, body) = mandelbrot(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"iterations\""));
    }

    #[test]
    fn test_mandelbrot_missing_params() {
        let state = fake_state();
        let req = req_from("/mandelbrot?width=10");
        let (code, _ctype, _body) = mandelbrot(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_mandelbrot_invalid_params() {
        let state = fake_state();
        let req = req_from("/mandelbrot?width=abc&height=10&max_iter=100");
        let (code, _ctype, _body) = mandelbrot(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_matrixmul_handler() {
        let state = fake_state();
        let req = req_from("/matrixmul?size=10&seed=123");
        let (code, _ctype, body) = matrixmul(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"result_sha256\""));
    }

    #[test]
    fn test_matrixmul_missing_size() {
        let state = fake_state();
        let req = req_from("/matrixmul?seed=123");
        let (code, _ctype, _body) = matrixmul(&state, &req);
        assert_eq!(code, 400);
    }

    // --------- PI (requiere el método "machin" que te pasé) ---------

    #[test]
    fn pi_digits_1_ok() {
        let state = fake_state();
        let req = req_from("/pi?digits=1");
        let (code, _ctype, body) = super::pi(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"digits\":1"));
        assert!(s.contains("\"method\":\"machin\""));
        assert!(s.contains("\"pi\":\"3.")); // 3.x
    }

    #[test]
    fn pi_digits_20_ok() {
        let state = fake_state();
        let req = req_from("/pi?digits=20");
        let (code, _ctype, body) = super::pi(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"digits\":20"));
        assert!(s.contains("\"method\":\"machin\""));
        // formato "3.<20 dígitos>"
        let start = s.find("\"pi\":\"").unwrap() + 6;
        let end = s[start..].find('"').unwrap() + start;
        let pi_str = &s[start..end];
        assert!(pi_str.starts_with("3."));
        assert_eq!(pi_str.len(), 1 + 1 + 20); // "3." + 20
    }

    #[test]
    fn pi_invalid_zero_and_too_large() {
        let state = fake_state();
        let req0 = req_from("/pi?digits=0");
        let (code0, _, _) = super::pi(&state, &req0);
        assert_eq!(code0, 400);

        let req_big = req_from("/pi?digits=2001");
        let (code_big, _, _) = super::pi(&state, &req_big);
        assert_eq!(code_big, 400);
    }

    // --------- MATRIXMUL ---------

    #[test]
    fn matrixmul_size_edges_ok() {
        // size=1 OK
        let state = fake_state();
        let req1 = req_from("/matrixmul?size=1&seed=123");
        let (code1, _ctype1, body1) = super::matrixmul(&state, &req1);
        assert_eq!(code1, 200);
        assert!(std::str::from_utf8(&body1).unwrap().contains("\"result_sha256\""));

        // size=1000 OK (si tu handler permite <=1000)
        let req2 = req_from("/matrixmul?size=1000&seed=321");
        let (code2, _ctype2, _body2) = super::matrixmul(&state, &req2);
        assert_eq!(code2, 200);
    }

    #[test]
    fn matrixmul_too_large_is_400() {
        let state = fake_state();
        let req = req_from("/matrixmul?size=1001&seed=1");
        let (code, _ctype, _body) = super::matrixmul(&state, &req);
        assert_eq!(code, 400);
    }


    #[test]
    fn test_matrixmul_invalid_size() {
        let state = fake_state();
        let req = req_from("/matrixmul?size=abc&seed=123");
        let (code, _ctype, _body) = matrixmul(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_matrixmul_too_large() {
        let state = fake_state();
        let req = req_from("/matrixmul?size=2000&seed=123");
        let (code, _ctype, _body) = matrixmul(&state, &req);
        assert_eq!(code, 400);
    }

    // Pruebas de funciones auxiliares
    #[test]
    fn test_is_prime_number() {
        assert!(is_prime_number(2));
        assert!(is_prime_number(3));
        assert!(is_prime_number(97));
        assert!(!is_prime_number(1));
        assert!(!is_prime_number(4));
        assert!(!is_prime_number(100));
    }

    #[test]
    fn test_factorize() {
        let factors = factorize(12);
        assert_eq!(factors.len(), 2);
        assert_eq!(factors[0], vec![2, 2]);
        assert_eq!(factors[1], vec![3, 1]);
    }

    #[test]
    fn test_factorize_prime() {
        let factors = factorize(97);
        assert_eq!(factors.len(), 1);
        assert_eq!(factors[0], vec![97, 1]);
    }

    #[test]
    fn test_calculate_pi_spigot() {
        let pi = calculate_pi_chudnovsky(5);
        assert!(pi.starts_with("3.141"));
    }

    #[test]
    fn test_calculate_mandelbrot() {
        let result = calculate_mandelbrot(5, 5, 10);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].len(), 5);
    }

    #[test]
    fn test_multiply_matrices() {
    let hash = multiply_matrices_sha256(3, 123);
    assert!(!hash.is_empty());
    assert_eq!(hash.len(), 64); // SHA-256 hex length
    }

    #[test]
    fn test_pseudo_random() {
        let val1 = pseudo_random(123);
        let val2 = pseudo_random(123);
        assert_eq!(val1, val2); // Deterministic
        
        let val3 = pseudo_random(124);
        assert_ne!(val1, val3); // Different seed, different result
    }
}