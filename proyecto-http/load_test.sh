#!/bin/bash

# Script de pruebas de carga y estrés para el servidor HTTP
# Verifica el rendimiento bajo diferentes condiciones de carga

BASE_URL="http://localhost:8080"
echo "🚀 INICIANDO PRUEBAS DE CARGA Y ESTRÉS"
echo "======================================"

# Función para verificar que el servidor esté corriendo
check_server() {
    echo "🔍 Verificando que el servidor esté corriendo..."
    if ! curl -s "$BASE_URL/status" > /dev/null; then
        echo "❌ Error: El servidor no está corriendo en $BASE_URL"
        echo "   Ejecuta: cargo run"
        exit 1
    fi
    echo "✅ Servidor está corriendo"
}

# Función para crear archivos de prueba grandes
create_large_files() {
    echo "📁 Creando archivos de prueba grandes..."
    
    # Crear directorio data si no existe
    mkdir -p data
    
    # Archivo de números grande (1MB)
    echo "Creando archivo de números grande (1MB)..."
    for i in {1..100000}; do
        echo $((RANDOM % 10000))
    done > data/large_numbers.txt
    
    # Archivo de texto grande (1MB)
    echo "Creando archivo de texto grande (1MB)..."
    for i in {1..10000}; do
        echo "Esta es la línea número $i con contenido de prueba para el servidor HTTP"
    done > data/large_text.txt
    
    echo "✅ Archivos de prueba creados"
}

# Función para prueba de carga básica
load_test_basic() {
    echo ""
    echo "🔥 PRUEBA DE CARGA BÁSICA"
    echo "=========================="
    
    echo "Probando endpoint /status con múltiples requests..."
    for i in {1..10}; do
        echo -n "Request $i: "
        start_time=$(date +%s%3N)
        response=$(curl -s -w "\nHTTP_CODE:%{http_code}\nTIME:%{time_total}" "$BASE_URL/status")
        end_time=$(date +%s%3N)
        duration=$((end_time - start_time))
        
        http_code=$(echo "$response" | grep "HTTP_CODE:" | cut -d: -f2)
        curl_time=$(echo "$response" | grep "TIME:" | cut -d: -f2)
        
        echo "HTTP $http_code, ${duration}ms (curl: ${curl_time}s)"
    done
}

# Función para prueba de carga concurrente
load_test_concurrent() {
    echo ""
    echo "⚡ PRUEBA DE CARGA CONCURRENTE"
    echo "============================="
    
    echo "Enviando 20 requests concurrentes a /timestamp..."
    start_time=$(date +%s%3N)
    
    # Crear array de PIDs para procesos en background
    pids=()
    
    for i in {1..20}; do
        (
            response=$(curl -s -w "\nHTTP_CODE:%{http_code}\nTIME:%{time_total}" "$BASE_URL/timestamp")
            echo "Concurrent request $i: $(echo "$response" | grep "HTTP_CODE:" | cut -d: -f2)"
        ) &
        pids+=($!)
    done
    
    # Esperar a que todos los procesos terminen
    for pid in "${pids[@]}"; do
        wait $pid
    done
    
    end_time=$(date +%s%3N)
    total_duration=$((end_time - start_time))
    echo "Total time for 20 concurrent requests: ${total_duration}ms"
}

# Función para prueba de CPU-bound
load_test_cpu_bound() {
    echo ""
    echo "🧮 PRUEBA DE CARGA CPU-BOUND"
    echo "============================"
    
    echo "Probando endpoints CPU-intensivos..."
    
    # Prueba de primalidad
    echo "Probando /isprime con números grandes..."
    for n in 97 997 9973 99991; do
        echo -n "isprime($n): "
        start_time=$(date +%s%3N)
        response=$(curl -s "$BASE_URL/isprime?n=$n")
        end_time=$(date +%s%3N)
        duration=$((end_time - start_time))
        
        if echo "$response" | grep -q "is_prime"; then
            echo "✅ ${duration}ms"
        else
            echo "❌ Error"
        fi
    done
    
    # Prueba de factorización
    echo "Probando /factor con números compuestos..."
    for n in 360 1000 10000; do
        echo -n "factor($n): "
        start_time=$(date +%s%3N)
        response=$(curl -s "$BASE_URL/factor?n=$n")
        end_time=$(date +%s%3N)
        duration=$((end_time - start_time))
        
        if echo "$response" | grep -q "factors"; then
            echo "✅ ${duration}ms"
        else
            echo "❌ Error"
        fi
    done
    
    # Prueba de cálculo de π
    echo "Probando /pi con diferentes dígitos..."
    for digits in 10 50 100; do
        echo -n "pi($digits): "
        start_time=$(date +%s%3N)
        response=$(curl -s "$BASE_URL/pi?digits=$digits")
        end_time=$(date +%s%3N)
        duration=$((end_time - start_time))
        
        if echo "$response" | grep -q "pi"; then
            echo "✅ ${duration}ms"
        else
            echo "❌ Error"
        fi
    done
}

# Función para prueba de IO-bound
load_test_io_bound() {
    echo ""
    echo "💾 PRUEBA DE CARGA IO-BOUND"
    echo "==========================="
    
    echo "Probando endpoints IO-intensivos..."
    
    # Prueba de ordenamiento
    echo "Probando /sortfile con archivo grande..."
    echo -n "sortfile(large_numbers.txt): "
    start_time=$(date +%s%3N)
    response=$(curl -s "$BASE_URL/sortfile?name=large_numbers.txt&algo=merge")
    end_time=$(date +%s%3N)
    duration=$((end_time - start_time))
    
    if echo "$response" | grep -q "sorted_file"; then
        echo "✅ ${duration}ms"
    else
        echo "❌ Error"
    fi
    
    # Prueba de conteo de palabras
    echo "Probando /wordcount con archivo grande..."
    echo -n "wordcount(large_text.txt): "
    start_time=$(date +%s%3N)
    response=$(curl -s "$BASE_URL/wordcount?name=large_text.txt")
    end_time=$(date +%s%3N)
    duration=$((end_time - start_time))
    
    if echo "$response" | grep -q "words"; then
        echo "✅ ${duration}ms"
    else
        echo "❌ Error"
    fi
    
    # Prueba de búsqueda
    echo "Probando /grep con archivo grande..."
    echo -n "grep(large_text.txt, 'línea'): "
    start_time=$(date +%s%3N)
    response=$(curl -s "$BASE_URL/grep?name=large_text.txt&pattern=línea")
    end_time=$(date +%s%3N)
    duration=$((end_time - start_time))
    
    if echo "$response" | grep -q "matches"; then
        echo "✅ ${duration}ms"
    else
        echo "❌ Error"
    fi
}

# Función para prueba de sistema de Jobs
load_test_jobs() {
    echo ""
    echo "🎯 PRUEBA DE CARGA SISTEMA DE JOBS"
    echo "=================================="
    
    echo "Enviando múltiples trabajos concurrentes..."
    
    # Enviar varios trabajos
    job_ids=()
    for i in {1..5}; do
        echo -n "Enviando trabajo $i: "
        response=$(curl -s "$BASE_URL/jobs/submit?task=isprime&n=$((97 + i * 100))&prio=normal")
        job_id=$(echo "$response" | grep -o '"job_id":"[^"]*"' | cut -d'"' -f4)
        if [ -n "$job_id" ]; then
            job_ids+=("$job_id")
            echo "✅ $job_id"
        else
            echo "❌ Error"
        fi
    done
    
    # Verificar estado de trabajos
    echo "Verificando estado de trabajos..."
    for job_id in "${job_ids[@]}"; do
        echo -n "Estado de $job_id: "
        response=$(curl -s "$BASE_URL/jobs/status?id=$job_id")
        status=$(echo "$response" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
        echo "$status"
    done
    
    # Esperar un poco y verificar resultados
    echo "Esperando 3 segundos para que se procesen..."
    sleep 3
    
    echo "Verificando resultados..."
    for job_id in "${job_ids[@]}"; do
        echo -n "Resultado de $job_id: "
        response=$(curl -s "$BASE_URL/jobs/result?id=$job_id")
        if echo "$response" | grep -q "is_prime"; then
            echo "✅ Completado"
        else
            echo "⏳ Pendiente o Error"
        fi
    done
}

# Función para prueba de estrés
stress_test() {
    echo ""
    echo "💥 PRUEBA DE ESTRÉS"
    echo "==================="
    
    echo "Enviando 50 requests rápidos a diferentes endpoints..."
    
    endpoints=(
        "/status"
        "/timestamp"
        "/reverse?text=stress_test"
        "/toupper?text=stress_test"
        "/random?count=5&min=1&max=100"
        "/hash?text=stress_test"
    )
    
    success_count=0
    error_count=0
    total_time=0
    
    start_time=$(date +%s%3N)
    
    for i in {1..50}; do
        endpoint=${endpoints[$((i % ${#endpoints[@]}))]}
        response=$(curl -s -w "\nHTTP_CODE:%{http_code}\nTIME:%{time_total}" "$BASE_URL$endpoint")
        http_code=$(echo "$response" | grep "HTTP_CODE:" | cut -d: -f2)
        curl_time=$(echo "$response" | grep "TIME:" | cut -d: -f2)
        
        if [ "$http_code" = "200" ]; then
            ((success_count++))
        else
            ((error_count++))
        fi
        
        total_time=$(echo "$total_time + $curl_time" | bc -l)
    done
    
    end_time=$(date +%s%3N)
    total_duration=$((end_time - start_time))
    
    echo "Resultados de prueba de estrés:"
    echo "  ✅ Requests exitosos: $success_count"
    echo "  ❌ Requests con error: $error_count"
    echo "  ⏱️  Tiempo total: ${total_duration}ms"
    echo "  📊 Requests/segundo: $(echo "scale=2; 50 * 1000 / $total_duration" | bc -l)"
    echo "  ⏱️  Tiempo promedio: $(echo "scale=2; $total_time / 50" | bc -l)s"
}

# Función para prueba de backpressure
backpressure_test() {
    echo ""
    echo "🚧 PRUEBA DE BACKPRESSURE"
    echo "========================="
    
    echo "Enviando muchos requests simultáneos para probar límites de cola..."
    
    # Enviar muchos requests CPU-intensivos simultáneos
    pids=()
    success_count=0
    error_count=0
    
    for i in {1..30}; do
        (
            response=$(curl -s "$BASE_URL/isprime?n=$((1000 + i))")
            if echo "$response" | grep -q "is_prime"; then
                echo "Request $i: ✅ Success"
            else
                echo "Request $i: ❌ Error or 503"
            fi
        ) &
        pids+=($!)
    done
    
    # Esperar a que todos terminen
    for pid in "${pids[@]}"; do
        wait $pid
    done
    
    echo "Prueba de backpressure completada"
}

# Función para generar reporte de métricas
metrics_report() {
    echo ""
    echo "📊 REPORTE DE MÉTRICAS"
    echo "======================"
    
    echo "Métricas del servidor:"
    curl -s "$BASE_URL/metrics" | jq '.' 2>/dev/null || curl -s "$BASE_URL/metrics"
    
    echo ""
    echo "Estado del servidor:"
    curl -s "$BASE_URL/status" | jq '.' 2>/dev/null || curl -s "$BASE_URL/status"
}

# Función para limpiar archivos de prueba
cleanup() {
    echo ""
    echo "🧹 LIMPIEZA"
    echo "==========="
    
    echo "Eliminando archivos de prueba..."
    rm -f data/large_numbers.txt
    rm -f data/large_text.txt
    rm -f data/large_numbers.txt.sorted
    rm -f data/large_text.txt.gz
    
    echo "✅ Limpieza completada"
}

# Función principal
main() {
    check_server
    create_large_files
    
    load_test_basic
    load_test_concurrent
    load_test_cpu_bound
    load_test_io_bound
    load_test_jobs
    stress_test
    backpressure_test
    metrics_report
    cleanup
    
    echo ""
    echo "🎉 PRUEBAS DE CARGA COMPLETADAS"
    echo "==============================="
    echo "Revisa los resultados arriba para evaluar el rendimiento del servidor."
    echo ""
    echo "📈 MÉTRICAS CLAVE A EVALUAR:"
    echo "- Tiempo de respuesta promedio"
    echo "- Requests por segundo"
    echo "- Tasa de errores"
    echo "- Comportamiento bajo carga"
    echo "- Efectividad del backpressure"
    echo "- Rendimiento de CPU vs IO-bound"
}

# Verificar dependencias
if ! command -v curl &> /dev/null; then
    echo "❌ Error: curl no está instalado"
    exit 1
fi

if ! command -v bc &> /dev/null; then
    echo "❌ Error: bc no está instalado (necesario para cálculos)"
    exit 1
fi

# Ejecutar pruebas
main
