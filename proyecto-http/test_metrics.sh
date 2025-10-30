#!/bin/bash

# Script de prueba para métricas por comando
# Ejecuta varias requests a diferentes endpoints y verifica /metrics

BASE_URL="http://localhost:8080"

echo "=========================================="
echo "Prueba de Métricas por Comando"
echo "=========================================="
echo ""

# Función para hacer request y mostrar resultado
test_endpoint() {
    local endpoint=$1
    local name=$2
    echo "Testing: $name"
    echo "GET $endpoint"
    curl -s "$BASE_URL$endpoint" > /dev/null
    echo "✓ Request enviada"
    echo ""
}

echo "1. Realizando requests a diferentes comandos..."
echo ""

# Básicos
test_endpoint "/status" "status"
test_endpoint "/timestamp" "timestamp"
test_endpoint "/reverse?text=hello" "reverse"
test_endpoint "/random?count=5&min=1&max=100" "random"

# CPU-bound
test_endpoint "/isprime?n=97" "isprime"
test_endpoint "/factor?n=360" "factor"
test_endpoint "/fibonacci?n=10" "fibonacci"
test_endpoint "/isprime?n=101" "isprime (otra vez)"
test_endpoint "/isprime?n=103" "isprime (tercera vez)"

# IO-bound (si tienes archivos de prueba)
# test_endpoint "/wordcount?name=test.txt" "wordcount"

echo ""
echo "2. Esperando 2 segundos para procesamiento..."
sleep 2

echo ""
echo "3. Consultando métricas detalladas..."
echo "GET /metrics"
echo ""

# Hacer request a metrics y formatear con jq si está disponible
METRICS_OUTPUT=$(curl -s "$BASE_URL/metrics")

if command -v jq &> /dev/null; then
    echo "$METRICS_OUTPUT" | jq '.'
else
    echo "$METRICS_OUTPUT"
    echo ""
    echo "TIP: Instala 'jq' para ver JSON formateado: sudo apt-get install jq"
fi

echo ""
echo "4. Verificando métricas específicas..."

# Verificar si isprime tiene métricas
if echo "$METRICS_OUTPUT" | grep -q '"isprime"'; then
    echo "✓ Métricas de 'isprime' encontradas"
else
    echo "✗ No se encontraron métricas de 'isprime'"
fi

# Verificar si factor tiene métricas
if echo "$METRICS_OUTPUT" | grep -q '"factor"'; then
    echo "✓ Métricas de 'factor' encontradas"
else
    echo "✗ No se encontraron métricas de 'factor'"
fi

# Verificar si tiene wait_ms y exec_ms
if echo "$METRICS_OUTPUT" | grep -q "avg_wait_ms"; then
    echo "✓ avg_wait_ms encontrado"
else
    echo "✗ avg_wait_ms NO encontrado"
fi

if echo "$METRICS_OUTPUT" | grep -q "avg_exec_ms"; then
    echo "✓ avg_exec_ms encontrado"
else
    echo "✗ avg_exec_ms NO encontrado"
fi

# Verificar percentiles
if echo "$METRICS_OUTPUT" | grep -q "\"p50\""; then
    echo "✓ Percentiles p50 encontrados"
else
    echo "✗ Percentiles p50 NO encontrados"
fi

if echo "$METRICS_OUTPUT" | grep -q "\"p95\""; then
    echo "✓ Percentiles p95 encontrados"
else
    echo "✗ Percentiles p95 NO encontrados"
fi

if echo "$METRICS_OUTPUT" | grep -q "\"p99\""; then
    echo "✓ Percentiles p99 encontrados"
else
    echo "✗ Percentiles p99 NO encontrados"
fi

echo ""
echo "=========================================="
echo "Prueba completada"
echo "=========================================="
echo ""
echo "Para ver métricas en tiempo real:"
echo "  curl $BASE_URL/metrics | jq"
echo ""
echo "Para hacer más requests y ver cómo cambian:"
echo "  for i in {1..10}; do curl -s '$BASE_URL/isprime?n=97' > /dev/null; done"
echo "  curl $BASE_URL/metrics | jq '.latency_ms.isprime'"

