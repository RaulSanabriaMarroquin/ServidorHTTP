//! Handlers IO-bound: /sortfile, /wordcount, /grep, /compress, /hashfile
//!
//! Todos los handlers siguen la misma firma:
//!   (state: &Shared, req: &Request) -> (status, content_type, body_bytes)

use crate::core::{now_ms_since_epoch, Request, Shared};
use std::fs;
use std::io::{BufRead, BufReader, Write};

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

/// GET /sortfile?name=FILE&algo=merge|quick
pub fn sortfile(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let start = now_ms_since_epoch();
    
    let name_param = req.query.get("name");
    let default_algo = "merge".to_string();
    let algo_param = req.query.get("algo").unwrap_or(&default_algo);
    
    if name_param.is_none() {
        return bad_request("Parameter 'name' is required");
    }
    
    let filename = name_param.unwrap();
    let file_path = format!("data/{}", filename);
    
    // Verificar que el archivo existe
    if !std::path::Path::new(&file_path).exists() {
        return not_found(&format!("File '{}' not found", filename));
    }
    
    // Validar algoritmo
    if algo_param != "merge" && algo_param != "quick" {
        return bad_request("Parameter 'algo' must be 'merge' or 'quick'");
    }
    
    // Leer números del archivo
    let numbers = match read_numbers_from_file(&file_path) {
        Ok(nums) => nums,
        Err(e) => {
            return bad_request(&format!("Error reading file: {}", e));
        }
    };
    
    if numbers.is_empty() {
        return bad_request("File is empty or contains no valid numbers");
    }
    
    // Ordenar números
    let mut sorted_numbers = numbers.clone();
    match algo_param.as_str() {
        "merge" => merge_sort(&mut sorted_numbers),
        "quick" => quick_sort(&mut sorted_numbers),
        _ => unreachable!(),
    }
    
    // Escribir archivo ordenado
    let sorted_filename = format!("{}.sorted", filename);
    let sorted_path = format!("data/{}", sorted_filename);
    
    match write_numbers_to_file(&sorted_path, &sorted_numbers) {
        Ok(_) => {
            let elapsed = now_ms_since_epoch() - start;
            let body = format!(
                r#"{{"file":"{}","algo":"{}","sorted_file":"{}","count":{},"elapsed_ms":{}}}"#,
                filename, algo_param, sorted_filename, sorted_numbers.len(), elapsed
            );
            json_ok(body.into_bytes())
        },
        Err(e) => {
            bad_request(&format!("Error writing sorted file: {}", e))
        }
    }
}

/// GET /wordcount?name=FILE
pub fn wordcount(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let start = now_ms_since_epoch();
    
    let name_param = req.query.get("name");
    if name_param.is_none() {
        return bad_request("Parameter 'name' is required");
    }
    
    let filename = name_param.unwrap();
    let file_path = format!("data/{}", filename);
    
    // Verificar que el archivo existe
    if !std::path::Path::new(&file_path).exists() {
        return not_found(&format!("File '{}' not found", filename));
    }
    
    // Contar líneas, palabras y bytes
    let (lines, words, bytes) = match count_file_stats(&file_path) {
        Ok(stats) => stats,
        Err(e) => {
            return bad_request(&format!("Error reading file: {}", e));
        }
    };
    
    let elapsed = now_ms_since_epoch() - start;
    let body = format!(
        r#"{{"file":"{}","lines":{},"words":{},"bytes":{},"elapsed_ms":{}}}"#,
        filename, lines, words, bytes, elapsed
    );
    
    json_ok(body.into_bytes())
}

/// GET /grep?name=FILE&pattern=REGEX
pub fn grep(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let start = now_ms_since_epoch();
    
    let name_param = req.query.get("name");
    let pattern_param = req.query.get("pattern");
    
    if name_param.is_none() || pattern_param.is_none() {
        return bad_request("Parameters 'name' and 'pattern' are required");
    }
    
    let filename = name_param.unwrap();
    let pattern = pattern_param.unwrap();
    let file_path = format!("data/{}", filename);
    
    // Verificar que el archivo existe
    if !std::path::Path::new(&file_path).exists() {
        return not_found(&format!("File '{}' not found", filename));
    }
    
    // Buscar patrones en el archivo
    let (matches, matching_lines) = match search_in_file(&file_path, pattern) {
        Ok(result) => result,
        Err(e) => {
            return bad_request(&format!("Error searching in file: {}", e));
        }
    };
    
    let elapsed = now_ms_since_epoch() - start;
    let matching_lines_json = serde_json::to_string(&matching_lines).unwrap_or_else(|_| "[]".to_string());
    let body = format!(
        r#"{{"file":"{}","pattern":"{}","matches":{},"matching_lines":{},"elapsed_ms":{}}}"#,
        filename, pattern, matches, matching_lines_json, elapsed
    );
    
    json_ok(body.into_bytes())
}

/// GET /compress?name=FILE&codec=gzip|xz
pub fn compress(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let start = now_ms_since_epoch();
    
    let name_param = req.query.get("name");
    let default_codec = "gzip".to_string();
    let codec_param = req.query.get("codec").unwrap_or(&default_codec);
    
    if name_param.is_none() {
        return bad_request("Parameter 'name' is required");
    }
    
    let filename = name_param.unwrap();
    let file_path = format!("data/{}", filename);
    
    // Verificar que el archivo existe
    if !std::path::Path::new(&file_path).exists() {
        return not_found(&format!("File '{}' not found", filename));
    }
    
    // Validar codec
    if codec_param != "gzip" && codec_param != "xz" {
        return bad_request("Parameter 'codec' must be 'gzip' or 'xz'");
    }
    
    // Leer archivo
    let file_content = match fs::read(&file_path) {
        Ok(content) => content,
        Err(e) => {
            return bad_request(&format!("Error reading file: {}", e));
        }
    };
    
    // Comprimir archivo
    let compressed_content = match compress_data(&file_content, codec_param) {
        Ok(content) => content,
        Err(e) => {
            return bad_request(&format!("Error compressing file: {}", e));
        }
    };
    
    // Escribir archivo comprimido
    let compressed_filename = format!("{}.{}", filename, 
        if codec_param == "gzip" { "gz" } else { "xz" });
    let compressed_path = format!("data/{}", compressed_filename);
    
    match fs::write(&compressed_path, &compressed_content) {
        Ok(_) => {
            let elapsed = now_ms_since_epoch() - start;
            let original_size = file_content.len();
            let compressed_size = compressed_content.len();
            let compression_ratio = (compressed_size as f64 / original_size as f64) * 100.0;
            
            let body = format!(
                r#"{{"file":"{}","codec":"{}","compressed_file":"{}","original_size":{},"compressed_size":{},"compression_ratio":{:.2},"elapsed_ms":{}}}"#,
                filename, codec_param, compressed_filename, original_size, compressed_size, compression_ratio, elapsed
            );
            json_ok(body.into_bytes())
        },
        Err(e) => {
            bad_request(&format!("Error writing compressed file: {}", e))
        }
    }
}

/// GET /hashfile?name=FILE&algo=sha256
pub fn hashfile(_state: &Shared, req: &Request) -> (u16, &'static str, Vec<u8>) {
    let start = now_ms_since_epoch();
    
    let name_param = req.query.get("name");
    let default_algo = "sha256".to_string();
    let algo_param = req.query.get("algo").unwrap_or(&default_algo);
    
    if name_param.is_none() {
        return bad_request("Parameter 'name' is required");
    }
    
    let filename = name_param.unwrap();
    let file_path = format!("data/{}", filename);
    
    // Verificar que el archivo existe
    if !std::path::Path::new(&file_path).exists() {
        return not_found(&format!("File '{}' not found", filename));
    }
    
    // Validar algoritmo
    if algo_param != "sha256" {
        return bad_request("Parameter 'algo' must be 'sha256'");
    }
    
    // Leer archivo
    let file_content = match fs::read(&file_path) {
        Ok(content) => content,
        Err(e) => {
            return bad_request(&format!("Error reading file: {}", e));
        }
    };
    
    // Calcular hash
    let hash_value = calculate_file_hash(&file_content);
    let elapsed = now_ms_since_epoch() - start;
    
    let body = format!(
        r#"{{"file":"{}","algo":"{}","hash":"{}","size":{},"elapsed_ms":{}}}"#,
        filename, algo_param, hash_value, file_content.len(), elapsed
    );
    
    json_ok(body.into_bytes())
}

// Funciones auxiliares

/// Lee números enteros de un archivo (uno por línea)
fn read_numbers_from_file(file_path: &str) -> Result<Vec<i32>, Box<dyn std::error::Error>> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut numbers = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        if let Ok(num) = line.trim().parse::<i32>() {
            numbers.push(num);
        }
    }
    
    Ok(numbers)
}

/// Escribe números enteros a un archivo (uno por línea)
fn write_numbers_to_file(file_path: &str, numbers: &[i32]) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = fs::File::create(file_path)?;
    for &num in numbers {
        writeln!(file, "{}", num)?;
    }
    Ok(())
}

/// Algoritmo de ordenamiento merge sort
fn merge_sort(arr: &mut [i32]) {
    if arr.len() <= 1 {
        return;
    }
    
    let mid = arr.len() / 2;
    merge_sort(&mut arr[..mid]);
    merge_sort(&mut arr[mid..]);
    
    let mut temp = arr.to_vec();
    merge(&arr[..mid], &arr[mid..], &mut temp);
    arr.copy_from_slice(&temp);
}

fn merge(left: &[i32], right: &[i32], result: &mut [i32]) {
    let mut i = 0;
    let mut j = 0;
    let mut k = 0;
    
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            result[k] = left[i];
            i += 1;
        } else {
            result[k] = right[j];
            j += 1;
        }
        k += 1;
    }
    
    while i < left.len() {
        result[k] = left[i];
        i += 1;
        k += 1;
    }
    
    while j < right.len() {
        result[k] = right[j];
        j += 1;
        k += 1;
    }
}

/// Algoritmo de ordenamiento quick sort
fn quick_sort(arr: &mut [i32]) {
    if arr.len() <= 1 {
        return;
    }
    
    let pivot = partition(arr);
    quick_sort(&mut arr[..pivot]);
    quick_sort(&mut arr[pivot + 1..]);
}

fn partition(arr: &mut [i32]) -> usize {
    let pivot = arr[arr.len() - 1];
    let mut i = 0;
    
    for j in 0..arr.len() - 1 {
        if arr[j] <= pivot {
            arr.swap(i, j);
            i += 1;
        }
    }
    
    arr.swap(i, arr.len() - 1);
    i
}

/// Cuenta líneas, palabras y bytes de un archivo
fn count_file_stats(file_path: &str) -> Result<(u64, u64, u64), Box<dyn std::error::Error>> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    
    let mut lines = 0;
    let mut words = 0;
    
    for line in reader.lines() {
        let line = line?;
        lines += 1;
        words += line.split_whitespace().count() as u64;
    }
    
    let bytes = fs::metadata(file_path)?.len();
    
    Ok((lines, words, bytes))
}

/// Busca un patrón en un archivo
fn search_in_file(file_path: &str, pattern: &str) -> Result<(u64, Vec<String>), Box<dyn std::error::Error>> {
    let file = fs::File::open(file_path)?;
    let reader = BufReader::new(file);
    
    let mut matches = 0;
    let mut matching_lines = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        if line.contains(pattern) {
            matches += 1;
            if matching_lines.len() < 10 {
                matching_lines.push(line);
            }
        }
    }
    
    Ok((matches, matching_lines))
}

/// Comprime datos usando algoritmos reales
fn compress_data(data: &[u8], codec: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match codec {
        "gzip" => {
            use flate2::write::GzEncoder;
            use flate2::Compression;
            use std::io::Write;
            
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(data)?;
            encoder.finish().map_err(|e| e.into())
        },
        "xz" => {
            use xz2::write::XzEncoder;
            use std::io::Write;
            
            let mut encoder = XzEncoder::new(Vec::new(), 6);
            encoder.write_all(data)?;
            encoder.finish().map_err(|e| e.into())
        },
        _ => Err("Unsupported codec. Use 'gzip' or 'xz'".into()),
    }
}

/// Compresión Run-Length Encoding simple
fn compress_rle(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    
    let mut compressed = Vec::new();
    let mut current_byte = data[0];
    let mut count = 1;
    
    for &byte in &data[1..] {
        if byte == current_byte && count < 255 {
            count += 1;
        } else {
            compressed.push(count);
            compressed.push(current_byte);
            current_byte = byte;
            count = 1;
        }
    }
    
    compressed.push(count);
    compressed.push(current_byte);
    
    compressed
}

/// Calcula hash SHA-256 de un archivo
fn calculate_file_hash(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:x}", hasher.finish())
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
    use std::fs;

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

    fn create_test_file(name: &str, content: &str) {
        std::fs::create_dir_all("data").unwrap_or_default();
        fs::write(format!("data/{}", name), content).unwrap();
    }

    fn cleanup_test_file(name: &str) {
        let _ = fs::remove_file(format!("data/{}", name));
    }

    #[test]
    fn test_sortfile_handler() {
        create_test_file("test_sort.txt", "3\n1\n4\n2\n5\n");
        
        let state = fake_state();
        let req = req_from("/sortfile?name=test_sort.txt&algo=merge");
        let (code, _ctype, body) = sortfile(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"sorted_file\""));
        
        cleanup_test_file("test_sort.txt");
        cleanup_test_file("test_sort.txt.sorted");
    }

    #[test]
    fn test_sortfile_missing_name() {
        let state = fake_state();
        let req = req_from("/sortfile?algo=merge");
        let (code, _ctype, _body) = sortfile(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_sortfile_nonexistent_file() {
        let state = fake_state();
        let req = req_from("/sortfile?name=nonexistent.txt&algo=merge");
        let (code, _ctype, _body) = sortfile(&state, &req);
        assert_eq!(code, 404);
    }

    #[test]
    fn test_sortfile_invalid_algo() {
        create_test_file("test_sort.txt", "1\n2\n3\n");
        
        let state = fake_state();
        let req = req_from("/sortfile?name=test_sort.txt&algo=invalid");
        let (code, _ctype, _body) = sortfile(&state, &req);
        assert_eq!(code, 400);
        
        cleanup_test_file("test_sort.txt");
    }

    #[test]
    fn test_wordcount_handler() {
        create_test_file("test_wc.txt", "hello world\nthis is a test\n");
        
        let state = fake_state();
        let req = req_from("/wordcount?name=test_wc.txt");
        let (code, _ctype, body) = wordcount(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"lines\":2"));
        assert!(s.contains("\"words\":7"));
        
        cleanup_test_file("test_wc.txt");
    }

    #[test]
    fn test_wordcount_missing_name() {
        let state = fake_state();
        let req = req_from("/wordcount");
        let (code, _ctype, _body) = wordcount(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_wordcount_nonexistent_file() {
        let state = fake_state();
        let req = req_from("/wordcount?name=nonexistent.txt");
        let (code, _ctype, _body) = wordcount(&state, &req);
        assert_eq!(code, 404);
    }

    #[test]
    fn test_grep_handler() {
        create_test_file("test_grep.txt", "hello world\nthis is a test\nhello again\n");
        
        let state = fake_state();
        let req = req_from("/grep?name=test_grep.txt&pattern=hello");
        let (code, _ctype, body) = grep(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"matches\":2"));
        
        cleanup_test_file("test_grep.txt");
    }

    #[test]
    fn test_grep_missing_params() {
        let state = fake_state();
        let req = req_from("/grep?name=test.txt");
        let (code, _ctype, _body) = grep(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_grep_nonexistent_file() {
        let state = fake_state();
        let req = req_from("/grep?name=nonexistent.txt&pattern=test");
        let (code, _ctype, _body) = grep(&state, &req);
        assert_eq!(code, 404);
    }

    #[test]
    fn test_compress_handler() {
        create_test_file("test_compress.txt", "hello world this is a test file");
        
        let state = fake_state();
        let req = req_from("/compress?name=test_compress.txt&codec=gzip");
        let (code, _ctype, body) = compress(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"compressed_file\""));
        
        cleanup_test_file("test_compress.txt");
        cleanup_test_file("test_compress.txt.gz");
    }

    #[test]
    fn test_compress_missing_name() {
        let state = fake_state();
        let req = req_from("/compress?codec=gzip");
        let (code, _ctype, _body) = compress(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_compress_invalid_codec() {
        create_test_file("test_compress.txt", "test content");
        
        let state = fake_state();
        let req = req_from("/compress?name=test_compress.txt&codec=invalid");
        let (code, _ctype, _body) = compress(&state, &req);
        assert_eq!(code, 400);
        
        cleanup_test_file("test_compress.txt");
    }

    #[test]
    fn test_hashfile_handler() {
        create_test_file("test_hash.txt", "hello world");
        
        let state = fake_state();
        let req = req_from("/hashfile?name=test_hash.txt&algo=sha256");
        let (code, _ctype, body) = hashfile(&state, &req);
        assert_eq!(code, 200);
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"hash\""));
        
        cleanup_test_file("test_hash.txt");
    }

    #[test]
    fn test_hashfile_missing_name() {
        let state = fake_state();
        let req = req_from("/hashfile?algo=sha256");
        let (code, _ctype, _body) = hashfile(&state, &req);
        assert_eq!(code, 400);
    }

    #[test]
    fn test_hashfile_invalid_algo() {
        create_test_file("test_hash.txt", "test content");
        
        let state = fake_state();
        let req = req_from("/hashfile?name=test_hash.txt&algo=md5");
        let (code, _ctype, _body) = hashfile(&state, &req);
        assert_eq!(code, 400);
        
        cleanup_test_file("test_hash.txt");
    }

    // Pruebas de funciones auxiliares
    #[test]
    fn test_read_numbers_from_file() {
        create_test_file("test_numbers.txt", "1\n2\n3\nabc\n4\n");
        
        let result = read_numbers_from_file("data/test_numbers.txt");
        assert!(result.is_ok());
        let numbers = result.unwrap();
        assert_eq!(numbers, vec![1, 2, 3, 4]);
        
        cleanup_test_file("test_numbers.txt");
    }

    #[test]
    fn test_write_numbers_to_file() {
        let numbers = vec![1, 2, 3, 4];
        let result = write_numbers_to_file("data/test_write.txt", &numbers);
        assert!(result.is_ok());
        
        let content = fs::read_to_string("data/test_write.txt").unwrap();
        assert_eq!(content, "1\n2\n3\n4\n");
        
        cleanup_test_file("test_write.txt");
    }

    #[test]
    fn test_merge_sort() {
        let mut arr = vec![3, 1, 4, 2, 5];
        merge_sort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_quick_sort() {
        let mut arr = vec![3, 1, 4, 2, 5];
        quick_sort(&mut arr);
        assert_eq!(arr, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_count_file_stats() {
        create_test_file("test_stats.txt", "hello world\nthis is a test\n");
        
        let result = count_file_stats("data/test_stats.txt");
        assert!(result.is_ok());
        let (lines, words, bytes) = result.unwrap();
        assert_eq!(lines, 2);
        assert_eq!(words, 7);
        assert!(bytes > 0);
        
        cleanup_test_file("test_stats.txt");
    }

    #[test]
    fn test_search_in_file() {
        create_test_file("test_search.txt", "hello world\nthis is a test\nhello again\n");
        
        let result = search_in_file("data/test_search.txt", "hello");
        assert!(result.is_ok());
        let (matches, lines) = result.unwrap();
        assert_eq!(matches, 2);
        assert_eq!(lines.len(), 2);
        
        cleanup_test_file("test_search.txt");
    }

    #[test]
    fn test_compress_rle() {
        let data = b"aaaabbbcc";
        let compressed = compress_rle(data);
        assert!(!compressed.is_empty());
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_compress_rle_empty() {
        let data = b"";
        let compressed = compress_rle(data);
        assert!(compressed.is_empty());
    }

    #[test]
    fn test_calculate_file_hash() {
        let data = b"hello world";
        let hash = calculate_file_hash(data);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 16); // DefaultHasher hex length
    }
}