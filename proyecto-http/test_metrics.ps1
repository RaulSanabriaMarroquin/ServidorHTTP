# Script de prueba para métricas por comando (PowerShell)
# Ejecuta varias requests a diferentes endpoints y verifica /metrics

$BASE_URL = "http://localhost:8080"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Prueba de Métricas por Comando" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

function Test-Endpoint {
    param(
        [string]$endpoint,
        [string]$name
    )
    Write-Host "Testing: $name" -ForegroundColor Yellow
    Write-Host "GET $endpoint"
    try {
        $response = Invoke-WebRequest -Uri "$BASE_URL$endpoint" -Method GET -UseBasicParsing -ErrorAction SilentlyContinue
        Write-Host "✓ Request enviada" -ForegroundColor Green
    } catch {
        Write-Host "✗ Error: $_" -ForegroundColor Red
    }
    Write-Host ""
}

Write-Host "1. Realizando requests a diferentes comandos..." -ForegroundColor Cyan
Write-Host ""

# Básicos
Test-Endpoint "/status" "status"
Test-Endpoint "/timestamp" "timestamp"
Test-Endpoint "/reverse?text=hello" "reverse"
Test-Endpoint "/random?count=5`&min=1`&max=100" "random"

# CPU-bound
Test-Endpoint "/isprime?n=97" "isprime"
Test-Endpoint "/factor?n=360" "factor"
Test-Endpoint "/fibonacci?n=10" "fibonacci"
Test-Endpoint "/isprime?n=101" "isprime (otra vez)"
Test-Endpoint "/isprime?n=103" "isprime (tercera vez)"

Write-Host ""
Write-Host "2. Esperando 2 segundos para procesamiento..." -ForegroundColor Cyan
Start-Sleep -Seconds 2

Write-Host ""
Write-Host "3. Consultando métricas detalladas..." -ForegroundColor Cyan
Write-Host "GET /metrics"
Write-Host ""

try {
    $metricsResponse = Invoke-RestMethod -Uri "$BASE_URL/metrics" -Method GET
    $metricsJson = $metricsResponse
    $metricsJsonString = $metricsJson | ConvertTo-Json -Depth 10
    Write-Host $metricsJsonString
} catch {
    Write-Host "Error obteniendo métricas: $_" -ForegroundColor Red
    try {
        $metricsText = (Invoke-WebRequest -Uri "$BASE_URL/metrics" -UseBasicParsing).Content
        Write-Host $metricsText
        $metricsJson = $metricsText | ConvertFrom-Json
    } catch {
        Write-Host "Error parseando métricas: $_" -ForegroundColor Red
        $metricsJson = @{}
    }
}

Write-Host ""
Write-Host "4. Verificando métricas específicas..." -ForegroundColor Cyan

# Verificar si isprime tiene métricas
if ($metricsJson.latency_ms.PSObject.Properties.Name -contains "isprime") {
    Write-Host "✓ Métricas de 'isprime' encontradas" -ForegroundColor Green
    $isprimeMetrics = $metricsJson.latency_ms.isprime
    Write-Host "  - count: $($isprimeMetrics.count)"
    Write-Host "  - avg_wait_ms: $($isprimeMetrics.avg_wait_ms)"
    Write-Host "  - avg_exec_ms: $($isprimeMetrics.avg_exec_ms)"
    Write-Host "  - p50: $($isprimeMetrics.p50)"
} else {
    Write-Host "✗ No se encontraron métricas de 'isprime'" -ForegroundColor Red
}

# Verificar si factor tiene métricas
if ($metricsJson.latency_ms.PSObject.Properties.Name -contains "factor") {
    Write-Host "✓ Métricas de 'factor' encontradas" -ForegroundColor Green
} else {
    Write-Host "✗ No se encontraron métricas de 'factor'" -ForegroundColor Red
}

# Verificar campos requeridos
$requiredFields = @("avg_wait_ms", "avg_exec_ms", "p50", "p95", "p99")
foreach ($field in $requiredFields) {
    $found = $false
    foreach ($cmd in $metricsJson.latency_ms.PSObject.Properties) {
        if ($cmd.Value.PSObject.Properties.Name -contains $field) {
            $found = $true
            break
        }
    }
    if ($found) {
        Write-Host "✓ Campo '$field' encontrado" -ForegroundColor Green
    } else {
        Write-Host "✗ Campo '$field' NO encontrado" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Prueba completada" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Para ver métricas en tiempo real:" -ForegroundColor Yellow
Write-Host "  Invoke-RestMethod '$BASE_URL/metrics' | ConvertTo-Json -Depth 10"
Write-Host ""
Write-Host "Para hacer más requests y ver cómo cambian:" -ForegroundColor Yellow
Write-Host "  1..10 | ForEach-Object { Invoke-WebRequest -Uri '$BASE_URL/isprime?n=97' -UseBasicParsing | Out-Null }"
Write-Host "  (Invoke-RestMethod '$BASE_URL/metrics').latency_ms.isprime"

