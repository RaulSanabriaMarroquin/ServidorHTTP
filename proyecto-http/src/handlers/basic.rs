//! Handlers para rutas livianas: /reverse, /toupper, /random, /timestamp, /hash, /help, /sleep, /simulate, /fibonacci, create/deletefile

use std::collections::HashMap;

/// Calcula el n-ésimo número de Fibonacci
/// 
/// # Argumentos
/// * `n` - El índice del número de Fibonacci a calcular
/// 
/// # Retorna
/// * `Some(u64)` - El número de Fibonacci si n es válido
/// * `None` - Si n es demasiado grande o inválido
pub fn fibonacci(n: u32) -> Option<u64> {
    // Limitar el cálculo para evitar overflow y tiempos muy largos
    if n > 93 {
        return None; // u64 overflow después de F(93)
    }
    
    if n == 0 {
        return Some(0);
    }
    if n == 1 {
        return Some(1);
    }
    
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    
    for _ in 2..=n {
        let temp = a + b;
        a = b;
        b = temp;
    }
    
    Some(b)
}

/// Handler para el endpoint /fibonacci
/// 
/// # Argumentos
/// * `query_params` - Parámetros de consulta de la URL
/// 
/// # Retorna
/// * `String` - Respuesta JSON con el resultado
pub fn handle_fibonacci(query_params: &HashMap<String, String>) -> String {
    // Extraer el parámetro 'n' de la query string
    let default_n = "10".to_string();
    let n_param = query_params.get("n").unwrap_or(&default_n);
    
    // Convertir a número
    let n = match n_param.parse::<u32>() {
        Ok(val) => val,
        Err(_) => {
            return serde_json::json!({
                "error": "invalid_parameter",
                "message": "Parameter 'n' must be a valid positive integer",
                "parameter": n_param
            }).to_string();
        }
    };
    
    // Calcular Fibonacci
    match fibonacci(n) {
        Some(result) => {
            serde_json::json!({
                "success": true,
                "n": n,
                "fibonacci": result,
                "message": format!("Fibonacci({}) = {}", n, result)
            }).to_string()
        },
        None => {
            serde_json::json!({
                "error": "calculation_error",
                "message": "Cannot calculate Fibonacci for n > 93 (would cause overflow)",
                "n": n
            }).to_string()
        }
    }
}

/// Handler para el endpoint /help
pub fn handle_help() -> String {
    serde_json::json!({
        "endpoints": {
            "/fibonacci": {
                "description": "Calculate the n-th Fibonacci number",
                "parameters": {
                    "n": "The index of the Fibonacci number to calculate (0-93)"
                },
                "example": "/fibonacci?n=10"
            },
            "/reverse": {
                "description": "Reverse a string",
                "parameters": {
                    "text": "The string to reverse"
                },
                "example": "/reverse?text=hello"
            },
            "/toupper": {
                "description": "Convert string to uppercase",
                "parameters": {
                    "text": "The string to convert"
                },
                "example": "/toupper?text=hello"
            },
            "/isprime": {
                "description": "Check if a number is prime",
                "parameters": {
                    "n": "The number to check"
                },
                "example": "/isprime?n=17"
            },
            "/sleep": {
                "description": "Sleep for specified milliseconds",
                "parameters": {
                    "ms": "Milliseconds to sleep (max 5000)"
                },
                "example": "/sleep?ms=1000"
            },
            "/status": "Server status information",
            "/timestamp": "Current server timestamp",
            "/help": "This help message"
        },
        "note": "All endpoints return JSON responses"
    }).to_string()
}

/// Handler para el endpoint /reverse
/// 
/// # Argumentos
/// * `query_params` - Parámetros de consulta de la URL
/// 
/// # Retorna
/// * `String` - Respuesta JSON con el texto invertido
pub fn handle_reverse(query_params: &HashMap<String, String>) -> String {
    let default_text = "hello".to_string();
    let text_param = query_params.get("text").unwrap_or(&default_text);
    
    if text_param.is_empty() {
        return serde_json::json!({
            "error": "invalid_parameter",
            "message": "Parameter 'text' cannot be empty",
            "parameter": text_param
        }).to_string();
    }
    
    let reversed: String = text_param.chars().rev().collect();
    
    serde_json::json!({
        "success": true,
        "original": text_param,
        "reversed": reversed,
        "message": format!("Reversed '{}' to '{}'", text_param, reversed)
    }).to_string()
}

/// Handler para el endpoint /toupper
/// 
/// # Argumentos
/// * `query_params` - Parámetros de consulta de la URL
/// 
/// # Retorna
/// * `String` - Respuesta JSON con el texto en mayúsculas
pub fn handle_toupper(query_params: &HashMap<String, String>) -> String {
    let default_text = "hello".to_string();
    let text_param = query_params.get("text").unwrap_or(&default_text);
    
    if text_param.is_empty() {
        return serde_json::json!({
            "error": "invalid_parameter",
            "message": "Parameter 'text' cannot be empty",
            "parameter": text_param
        }).to_string();
    }
    
    let uppercase = text_param.to_uppercase();
    
    serde_json::json!({
        "success": true,
        "original": text_param,
        "uppercase": uppercase,
        "message": format!("Converted '{}' to '{}'", text_param, uppercase)
    }).to_string()
}

/// Handler para el endpoint /isprime
/// 
/// # Argumentos
/// * `query_params` - Parámetros de consulta de la URL
/// 
/// # Retorna
/// * `String` - Respuesta JSON indicando si el número es primo
pub fn handle_isprime(query_params: &HashMap<String, String>) -> String {
    let default_n = "17".to_string();
    let n_param = query_params.get("n").unwrap_or(&default_n);
    
    // Convertir a número
    let n = match n_param.parse::<u64>() {
        Ok(val) => val,
        Err(_) => {
            return serde_json::json!({
                "error": "invalid_parameter",
                "message": "Parameter 'n' must be a valid positive integer",
                "parameter": n_param
            }).to_string();
        }
    };
    
    // Verificar límites razonables
    if n > 1_000_000 {
        return serde_json::json!({
            "error": "parameter_too_large",
            "message": "Parameter 'n' must be <= 1,000,000 for performance reasons",
            "n": n
        }).to_string();
    }
    
    let is_prime = is_prime_number(n);
    
    serde_json::json!({
        "success": true,
        "n": n,
        "is_prime": is_prime,
        "message": format!("{} is {}", n, if is_prime { "prime" } else { "not prime" })
    }).to_string()
}

/// Handler para el endpoint /sleep
/// 
/// # Argumentos
/// * `query_params` - Parámetros de consulta de la URL
/// 
/// # Retorna
/// * `String` - Respuesta JSON después de dormir
pub fn handle_sleep(query_params: &HashMap<String, String>) -> String {
    let default_ms = "1000".to_string();
    let ms_param = query_params.get("ms").unwrap_or(&default_ms);
    
    // Convertir a número
    let ms = match ms_param.parse::<u64>() {
        Ok(val) => val,
        Err(_) => {
            return serde_json::json!({
                "error": "invalid_parameter",
                "message": "Parameter 'ms' must be a valid positive integer",
                "parameter": ms_param
            }).to_string();
        }
    };
    
    // Limitar el tiempo de sleep para evitar abuso
    if ms > 5000 {
        return serde_json::json!({
            "error": "parameter_too_large",
            "message": "Parameter 'ms' must be <= 5000 milliseconds",
            "ms": ms
        }).to_string();
    }
    
    // Dormir por el tiempo especificado
    std::thread::sleep(std::time::Duration::from_millis(ms));
    
    serde_json::json!({
        "success": true,
        "slept_ms": ms,
        "message": format!("Slept for {} milliseconds", ms)
    }).to_string()
}

/// Función auxiliar para verificar si un número es primo
/// 
/// # Argumentos
/// * `n` - El número a verificar
/// 
/// # Retorna
/// * `bool` - true si es primo, false si no
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