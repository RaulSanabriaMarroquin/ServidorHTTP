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

/// Handler para el endpoint /random
/// 
/// # Argumentos
/// * `query_params` - Parámetros de consulta de la URL
/// 
/// # Retorna
/// * `String` - Respuesta JSON con números aleatorios
pub fn handle_random(query_params: &HashMap<String, String>) -> String {
    let default_count = "5".to_string();
    let default_min = "1".to_string();
    let default_max = "100".to_string();
    
    let count_param = query_params.get("count").unwrap_or(&default_count);
    let min_param = query_params.get("min").unwrap_or(&default_min);
    let max_param = query_params.get("max").unwrap_or(&default_max);
    
    // Convertir parámetros a números
    let count = match count_param.parse::<u32>() {
        Ok(val) => val,
        Err(_) => {
            return serde_json::json!({
                "error": "invalid_parameter",
                "message": "Parameter 'count' must be a valid positive integer",
                "parameter": count_param
            }).to_string();
        }
    };
    
    let min_val = match min_param.parse::<i32>() {
        Ok(val) => val,
        Err(_) => {
            return serde_json::json!({
                "error": "invalid_parameter",
                "message": "Parameter 'min' must be a valid integer",
                "parameter": min_param
            }).to_string();
        }
    };
    
    let max_val = match max_param.parse::<i32>() {
        Ok(val) => val,
        Err(_) => {
            return serde_json::json!({
                "error": "invalid_parameter",
                "message": "Parameter 'max' must be a valid integer",
                "parameter": max_param
            }).to_string();
        }
    };
    
    // Validaciones
    if count == 0 || count > 1000 {
        return serde_json::json!({
            "error": "invalid_parameter",
            "message": "Parameter 'count' must be between 1 and 1000",
            "count": count
        }).to_string();
    }
    
    if min_val >= max_val {
        return serde_json::json!({
            "error": "invalid_parameter",
            "message": "Parameter 'min' must be less than 'max'",
            "min": min_val,
            "max": max_val
        }).to_string();
    }
    
    // Generar números aleatorios
    let mut numbers = Vec::new();
    
    for i in 0..count {
        // Usar un seed simple basado en el tiempo y el índice
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() + i as u128;
        
        // Generar número pseudoaleatorio simple
        let range = max_val - min_val + 1;
        let random_num = min_val + ((seed % range as u128) as i32);
        numbers.push(random_num);
    }
    
    serde_json::json!({
        "success": true,
        "count": count,
        "min": min_val,
        "max": max_val,
        "numbers": numbers,
        "message": format!("Generated {} random numbers between {} and {}", count, min_val, max_val)
    }).to_string()
}

/// Handler para el endpoint /hash
/// 
/// # Argumentos
/// * `query_params` - Parámetros de consulta de la URL
/// 
/// # Retorna
/// * `String` - Respuesta JSON con el hash del texto
pub fn handle_hash(query_params: &HashMap<String, String>) -> String {
    let default_text = "hello world".to_string();
    let text_param = query_params.get("text").unwrap_or(&default_text);
    
    if text_param.is_empty() {
        return serde_json::json!({
            "error": "invalid_parameter",
            "message": "Parameter 'text' cannot be empty",
            "parameter": text_param
        }).to_string();
    }
    
    // Calcular hash simple usando el hash del string
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    text_param.hash(&mut hasher);
    let hash_value = hasher.finish();
    
    serde_json::json!({
        "success": true,
        "text": text_param,
        "hash": format!("{:x}", hash_value),
        "message": format!("Hash calculated for '{}'", text_param)
    }).to_string()
}

/// Handler para el endpoint /simulate
/// 
/// # Argumentos
/// * `query_params` - Parámetros de consulta de la URL
/// 
/// # Retorna
/// * `String` - Respuesta JSON después de simular
pub fn handle_simulate(query_params: &HashMap<String, String>) -> String {
    let default_seconds = "2".to_string();
    let default_task = "cpu_intensive".to_string();
    
    let seconds_param = query_params.get("seconds").unwrap_or(&default_seconds);
    let task_param = query_params.get("task").unwrap_or(&default_task);
    
    // Convertir segundos a número
    let seconds = match seconds_param.parse::<u64>() {
        Ok(val) => val,
        Err(_) => {
            return serde_json::json!({
                "error": "invalid_parameter",
                "message": "Parameter 'seconds' must be a valid positive integer",
                "parameter": seconds_param
            }).to_string();
        }
    };
    
    // Limitar el tiempo de simulación
    if seconds > 10 {
        return serde_json::json!({
            "error": "parameter_too_large",
            "message": "Parameter 'seconds' must be <= 10",
            "seconds": seconds
        }).to_string();
    }
    
    // Simular trabajo según el tipo de tarea
    match task_param.as_str() {
        "cpu_intensive" => {
            // Simular trabajo CPU-intensivo
            let start = std::time::Instant::now();
            let mut result = 0u64;
            for i in 0..(seconds * 1_000_000) {
                result += i;
            }
            let elapsed = start.elapsed().as_millis();
            
            serde_json::json!({
                "success": true,
                "task": task_param,
                "seconds": seconds,
                "result": result,
                "elapsed_ms": elapsed,
                "message": format!("Simulated {} for {} seconds", task_param, seconds)
            }).to_string()
        },
        "io_intensive" => {
            // Simular trabajo IO-intensivo
            std::thread::sleep(std::time::Duration::from_secs(seconds));
            
            serde_json::json!({
                "success": true,
                "task": task_param,
                "seconds": seconds,
                "message": format!("Simulated {} for {} seconds", task_param, seconds)
            }).to_string()
        },
        _ => {
            serde_json::json!({
                "error": "invalid_parameter",
                "message": "Parameter 'task' must be 'cpu_intensive' or 'io_intensive'",
                "task": task_param
            }).to_string()
        }
    }
}

/// Handler para el endpoint /loadtest
/// 
/// # Argumentos
/// * `query_params` - Parámetros de consulta de la URL
/// 
/// # Retorna
/// * `String` - Respuesta JSON con resultados de la prueba de carga
pub fn handle_loadtest(query_params: &HashMap<String, String>) -> String {
    let default_tasks = "10".to_string();
    let default_sleep = "100".to_string();
    
    let tasks_param = query_params.get("tasks").unwrap_or(&default_tasks);
    let sleep_param = query_params.get("sleep").unwrap_or(&default_sleep);
    
    // Convertir parámetros a números
    let tasks = match tasks_param.parse::<u32>() {
        Ok(val) => val,
        Err(_) => {
            return serde_json::json!({
                "error": "invalid_parameter",
                "message": "Parameter 'tasks' must be a valid positive integer",
                "parameter": tasks_param
            }).to_string();
        }
    };
    
    let sleep_ms = match sleep_param.parse::<u64>() {
        Ok(val) => val,
        Err(_) => {
            return serde_json::json!({
                "error": "invalid_parameter",
                "message": "Parameter 'sleep' must be a valid positive integer",
                "parameter": sleep_param
            }).to_string();
        }
    };
    
    // Validaciones
    if tasks == 0 || tasks > 100 {
        return serde_json::json!({
            "error": "invalid_parameter",
            "message": "Parameter 'tasks' must be between 1 and 100",
            "tasks": tasks
        }).to_string();
    }
    
    if sleep_ms > 1000 {
        return serde_json::json!({
            "error": "parameter_too_large",
            "message": "Parameter 'sleep' must be <= 1000 milliseconds",
            "sleep": sleep_ms
        }).to_string();
    }
    
    // Ejecutar prueba de carga
    let start = std::time::Instant::now();
    let mut results = Vec::new();
    
    for _i in 0..tasks {
        let task_start = std::time::Instant::now();
        
        // Simular trabajo
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
        
        let task_elapsed = task_start.elapsed().as_millis();
        results.push(task_elapsed);
    }
    
    let total_elapsed = start.elapsed().as_millis();
    let avg_elapsed = results.iter().sum::<u128>() / tasks as u128;
    
    serde_json::json!({
        "success": true,
        "tasks": tasks,
        "sleep_ms": sleep_ms,
        "total_elapsed_ms": total_elapsed,
        "avg_task_elapsed_ms": avg_elapsed,
        "results": results,
        "message": format!("Completed {} tasks with {}ms sleep each", tasks, sleep_ms)
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
            "/random": {
                "description": "Generate random numbers",
                "parameters": {
                    "count": "Number of random numbers (1-1000)",
                    "min": "Minimum value",
                    "max": "Maximum value"
                },
                "example": "/random?count=5&min=1&max=100"
            },
            "/hash": {
                "description": "Calculate hash of text",
                "parameters": {
                    "text": "The text to hash"
                },
                "example": "/hash?text=hello"
            },
            "/simulate": {
                "description": "Simulate CPU or IO intensive task",
                "parameters": {
                    "seconds": "Duration in seconds (max 10)",
                    "task": "Task type: cpu_intensive or io_intensive"
                },
                "example": "/simulate?seconds=2&task=cpu_intensive"
            },
            "/loadtest": {
                "description": "Run load test",
                "parameters": {
                    "tasks": "Number of tasks (1-100)",
                    "sleep": "Sleep time per task in ms (max 1000)"
                },
                "example": "/loadtest?tasks=10&sleep=100"
            },
            "/createfile": {
                "description": "Create a file with content",
                "parameters": {
                    "name": "Filename",
                    "content": "File content",
                    "repeat": "Repeat content X times"
                },
                "example": "/createfile?name=test.txt&content=hello&repeat=3"
            },
            "/deletefile": {
                "description": "Delete a file",
                "parameters": {
                    "name": "Filename to delete"
                },
                "example": "/deletefile?name=test.txt"
            },
            "/status": "Server status information",
            "/timestamp": "Current server timestamp",
            "/help": "This help message"
        },
        "note": "All endpoints return JSON responses"
    }).to_string()
}

/// Handler para el endpoint /createfile
/// 
/// # Argumentos
/// * `query_params` - Parámetros de consulta de la URL
/// 
/// # Retorna
/// * `String` - Respuesta JSON con resultado de creación de archivo
pub fn handle_createfile(query_params: &HashMap<String, String>) -> String {
    let name_param = query_params.get("name");
    let content_param = query_params.get("content");
    let default_repeat = "1".to_string();
    let repeat_param = query_params.get("repeat").unwrap_or(&default_repeat);
    
    // Validar parámetros requeridos
    if name_param.is_none() {
        return serde_json::json!({
            "error": "missing_parameter",
            "message": "Parameter 'name' is required",
            "required": ["name", "content"]
        }).to_string();
    }
    
    if content_param.is_none() {
        return serde_json::json!({
            "error": "missing_parameter",
            "message": "Parameter 'content' is required",
            "required": ["name", "content"]
        }).to_string();
    }
    
    let filename = name_param.unwrap();
    let content = content_param.unwrap();
    
    // Convertir repeat a número
    let repeat = match repeat_param.parse::<u32>() {
        Ok(val) => val,
        Err(_) => {
            return serde_json::json!({
                "error": "invalid_parameter",
                "message": "Parameter 'repeat' must be a valid positive integer",
                "parameter": repeat_param
            }).to_string();
        }
    };
    
    // Validaciones
    if filename.is_empty() {
        return serde_json::json!({
            "error": "invalid_parameter",
            "message": "Parameter 'name' cannot be empty"
        }).to_string();
    }
    
    if repeat == 0 || repeat > 1000 {
        return serde_json::json!({
            "error": "invalid_parameter",
            "message": "Parameter 'repeat' must be between 1 and 1000",
            "repeat": repeat
        }).to_string();
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
    match std::fs::write(&file_path, file_content) {
        Ok(_) => {
            let file_size = std::fs::metadata(&file_path)
                .map(|m| m.len())
                .unwrap_or(0);
            
            serde_json::json!({
                "success": true,
                "filename": filename,
                "file_path": file_path,
                "content_length": content.len(),
                "repeat": repeat,
                "file_size_bytes": file_size,
                "message": format!("Created file '{}' with {} repetitions", filename, repeat)
            }).to_string()
        },
        Err(e) => {
            serde_json::json!({
                "error": "file_error",
                "message": format!("Failed to create file: {}", e),
                "filename": filename
            }).to_string()
        }
    }
}

/// Handler para el endpoint /deletefile
/// 
/// # Argumentos
/// * `query_params` - Parámetros de consulta de la URL
/// 
/// # Retorna
/// * `String` - Respuesta JSON con resultado de eliminación de archivo
pub fn handle_deletefile(query_params: &HashMap<String, String>) -> String {
    let name_param = query_params.get("name");
    
    // Validar parámetro requerido
    if name_param.is_none() {
        return serde_json::json!({
            "error": "missing_parameter",
            "message": "Parameter 'name' is required",
            "required": ["name"]
        }).to_string();
    }
    
    let filename = name_param.unwrap();
    
    if filename.is_empty() {
        return serde_json::json!({
            "error": "invalid_parameter",
            "message": "Parameter 'name' cannot be empty"
        }).to_string();
    }
    
    // Intentar eliminar archivo
    let file_path = format!("data/{}", filename);
    match std::fs::remove_file(&file_path) {
        Ok(_) => {
            serde_json::json!({
                "success": true,
                "filename": filename,
                "file_path": file_path,
                "message": format!("Deleted file '{}'", filename)
            }).to_string()
        },
        Err(e) => {
            serde_json::json!({
                "error": "file_error",
                "message": format!("Failed to delete file: {}", e),
                "filename": filename,
                "file_path": file_path
            }).to_string()
        }
    }
}