#!/bin/bash

# Script de pruebas completo para el servidor HTTP
# Verifica todos los endpoints según el documento del proyecto

BASE_URL="http://localhost:8080"
echo "🧪 INICIANDO PRUEBAS COMPLETAS DEL SERVIDOR HTTP"
echo "=================================================="

# Función para hacer requests y mostrar resultados
test_endpoint() {
    local endpoint="$1"
    local description="$2"
    echo ""
    echo "🔍 Probando: $description"
    echo "URL: $BASE_URL$endpoint"
    
    response=$(curl -s -w "\nHTTP_CODE:%{http_code}\nTIME:%{time_total}" "$BASE_URL$endpoint")
    echo "Respuesta:"
    echo "$response" | head -n -2
    echo ""
}

# Función para crear archivos de prueba
create_test_files() {
    echo "📁 Creando archivos de prueba..."
    
    # Crear directorio data si no existe
    mkdir -p data
    
    # Crear archivo de números para pruebas de ordenamiento
    echo "Creando archivo de números..."
    for i in {1..1000}; do
        echo $((RANDOM % 1000))
    done > data/numbers.txt
    
    # Crear archivo de texto para pruebas de wordcount y grep
    echo "Creando archivo de texto..."
    cat > data/sample.txt << EOF
Este es un archivo de prueba para el servidor HTTP.
Contiene múltiples líneas con diferentes palabras.
Algunas líneas tienen números como 123 y 456.
También hay caracteres especiales: @#$%^&*()
El objetivo es probar las funciones de wordcount y grep.
EOF
    
    echo "✅ Archivos de prueba creados"
}

# Función para probar el sistema de Jobs
test_jobs_system() {
    echo ""
    echo "🎯 PROBANDO SISTEMA DE JOBS"
    echo "============================"
    
    # Enviar trabajo
    echo "Enviando trabajo de prueba..."
    job_response=$(curl -s "$BASE_URL/jobs/submit?task=isprime&n=97&prio=high")
    echo "Respuesta de submit: $job_response"
    
    # Extraer job_id
    job_id=$(echo "$job_response" | grep -o '"job_id":"[^"]*"' | cut -d'"' -f4)
    echo "Job ID: $job_id"
    
    if [ -n "$job_id" ]; then
        # Verificar estado
        echo "Verificando estado del trabajo..."
        curl -s "$BASE_URL/jobs/status?id=$job_id"
        echo ""
        
        # Esperar un poco
        echo "Esperando 2 segundos..."
        sleep 2
        
        # Verificar estado nuevamente
        echo "Verificando estado después de esperar..."
        curl -s "$BASE_URL/jobs/status?id=$job_id"
        echo ""
        
        # Obtener resultado
        echo "Obteniendo resultado..."
        curl -s "$BASE_URL/jobs/result?id=$job_id"
        echo ""
    fi
}

# Crear archivos de prueba
create_test_files

echo ""
echo "🚀 INICIANDO PRUEBAS DE ENDPOINTS"
echo "================================="

# Endpoints básicos
test_endpoint "/status" "Estado del servidor"
test_endpoint "/timestamp" "Timestamp actual"
test_endpoint "/help" "Ayuda del sistema"

# Endpoints de texto
test_endpoint "/reverse?text=hello" "Invertir texto"
test_endpoint "/toupper?text=hello" "Convertir a mayúsculas"
test_endpoint "/hash?text=hello" "Calcular hash"

# Endpoints de números
test_endpoint "/random?count=5&min=1&max=100" "Números aleatorios"
test_endpoint "/fibonacci?n=10" "Números de Fibonacci"
test_endpoint "/isprime?n=97" "Verificar número primo"

# Endpoints CPU-bound
test_endpoint "/factor?n=360" "Factorización"
test_endpoint "/pi?digits=10" "Cálculo de π"
test_endpoint "/mandelbrot?width=10&height=10&max_iter=100" "Conjunto de Mandelbrot"
test_endpoint "/matrixmul?size=10&seed=123" "Multiplicación de matrices"

# Endpoints de simulación
test_endpoint "/simulate?seconds=1&task=cpu_intensive" "Simulación CPU"
test_endpoint "/sleep?ms=1000" "Sleep"
test_endpoint "/loadtest?tasks=3&sleep=100" "Prueba de carga"

# Endpoints de archivos
test_endpoint "/createfile?name=test.txt&content=hello&repeat=3" "Crear archivo"
test_endpoint "/deletefile?name=test.txt" "Eliminar archivo"

# Endpoints IO-bound
test_endpoint "/sortfile?name=numbers.txt&algo=merge" "Ordenar archivo"
test_endpoint "/wordcount?name=sample.txt" "Contar palabras"
test_endpoint "/grep?name=sample.txt&pattern=número" "Buscar en archivo"
test_endpoint "/compress?name=sample.txt&codec=gzip" "Comprimir archivo"
test_endpoint "/hashfile?name=sample.txt&algo=sha256" "Hash de archivo"

# Endpoint de métricas
test_endpoint "/metrics" "Métricas del sistema"

# Probar sistema de Jobs
test_jobs_system

echo ""
echo "🎉 PRUEBAS COMPLETADAS"
echo "======================"
echo "Revisa las respuestas arriba para verificar que todos los endpoints funcionan correctamente."
echo ""
echo "📊 RESUMEN DE ENDPOINTS PROBADOS:"
echo "- Básicos: /status, /timestamp, /help"
echo "- Texto: /reverse, /toupper, /hash"
echo "- Números: /random, /fibonacci, /isprime"
echo "- CPU-bound: /factor, /pi, /mandelbrot, /matrixmul"
echo "- Simulación: /simulate, /sleep, /loadtest"
echo "- Archivos: /createfile, /deletefile"
echo "- IO-bound: /sortfile, /wordcount, /grep, /compress, /hashfile"
echo "- Sistema: /metrics"
echo "- Jobs: /jobs/submit, /jobs/status, /jobs/result"
echo ""
echo "Total: 25+ endpoints implementados y probados"
