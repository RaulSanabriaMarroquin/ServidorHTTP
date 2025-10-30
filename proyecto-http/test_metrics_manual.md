# Guía de Pruebas - Métricas por Comando

## Requisitos Previos

1. **Servidor ejecutándose:**
   ```bash
   cargo run
   # o en Windows PowerShell:
   cargo run
   ```

2. **Herramientas recomendadas:**
   - `curl` (Linux/macOS) o `Invoke-WebRequest` (PowerShell)
   - `jq` para formatear JSON (opcional pero recomendado)

## Pasos de Prueba

### 1. Iniciar el Servidor

```bash
cd proyecto-http
cargo run
```

Deberías ver:
```
[INFO] listening on http://0.0.0.0:8080 (HTTP/1.0)
```

### 2. Ejecutar Script de Prueba Automático

#### En Linux/macOS:
```bash
chmod +x test_metrics.sh
./test_metrics.sh
```

#### En Windows PowerShell:
```powershell
.\test_metrics.ps1
```

### 3. Prueba Manual - Paso a Paso

#### A. Hacer Requests a Diferentes Comandos

```bash
# Básicos
curl http://localhost:8080/status
curl http://localhost:8080/timestamp
curl "http://localhost:8080/reverse?text=hello"
curl "http://localhost:8080/random?count=5&min=1&max=100"

# CPU-bound (estos toman más tiempo)
curl "http://localhost:8080/isprime?n=97"
curl "http://localhost:8080/factor?n=360"
curl "http://localhost:8080/fibonacci?n=10"

# Más requests del mismo comando para ver estadísticas
curl "http://localhost:8080/isprime?n=101"
curl "http://localhost:8080/isprime?n=103"
curl "http://localhost:8080/isprime?n=107"
```

#### B. Consultar Métricas

```bash
# Ver métricas completas (formateadas con jq)
curl http://localhost:8080/metrics | jq

# O sin jq (menos legible)
curl http://localhost:8080/metrics
```

#### C. Verificar Métricas Específicas por Comando

```bash
# Ver solo métricas de 'isprime'
curl http://localhost:8080/metrics | jq '.latency_ms.isprime'

# Ver colas por comando
curl http://localhost:8080/metrics | jq '.queues'

# Ver workers por comando
curl http://localhost:8080/metrics | jq '.workers'

# Ver métricas completas de un comando
curl http://localhost:8080/metrics | jq '.latency_ms.isprime | {count, avg_wait_ms, avg_exec_ms, p50, p95, p99, stddev_wait_ms, stddev_exec_ms}'
```

### 4. Prueba de Carga - Generar Más Datos

```bash
# Generar 20 requests a isprime para ver estadísticas
for i in {1..20}; do
  curl -s "http://localhost:8080/isprime?n=$((97 + i))" > /dev/null
  echo "Request $i enviada"
done

# Esperar un momento
sleep 2

# Ver métricas actualizadas
curl http://localhost:8080/metrics | jq '.latency_ms.isprime'
```

En PowerShell:
```powershell
1..20 | ForEach-Object {
  $num = 97 + $_
  Invoke-WebRequest -Uri "http://localhost:8080/isprime?n=$num" -UseBasicParsing | Out-Null
  Write-Host "Request $_ enviada"
}
Start-Sleep -Seconds 2
(Invoke-RestMethod http://localhost:8080/metrics).latency_ms.isprime
```

### 5. Verificar Campos Requeridos

Verifica que el JSON de `/metrics` tenga esta estructura:

```json
{
  "queues": {
    "isprime": 0,
    "factor": 0,
    ...
  },
  "workers": {
    "isprime": {
      "total": 4,
      "busy": 0
    },
    ...
  },
  "latency_ms": {
    "isprime": {
      "count": 5,
      "avg_wait_ms": 2.5,
      "avg_exec_ms": 15.3,
      "stddev_wait_ms": 1.2,
      "stddev_exec_ms": 3.4,
      "p50": 12,
      "p95": 20,
      "p99": 25
    },
    ...
  }
}
```

### 6. Verificar que los Tiempos Tienen Sentido

- `wait_ms` debería ser pequeño (0-50ms típicamente) - tiempo en cola
- `exec_ms` debería reflejar el tiempo real de ejecución del comando
- `p50 < p95 < p99` (percentiles deben ser crecientes)

```bash
# Ver distribución de tiempos
curl http://localhost:8080/metrics | jq '.latency_ms.isprime'
```

### 7. Prueba de Múltiples Comandos Simultáneos

```bash
# Ejecutar múltiples requests en paralelo
for i in {1..10}; do
  curl -s "http://localhost:8080/isprime?n=$((100 + iмен))" > /dev/null &
  curl -s "http://localhost:8080/factor?n=$((200 + $i))" > /dev/null &
done

wait
sleep 2
curl http://localhost:8080/metrics | jq '{isprime: .latency_ms.isprime, factor: .latency_ms.factor}'
```

## Qué Verificar

### ✅ Campos Requeridos
- [ ] `queues` por comando
- [ ] `workers` por comando (total, busy)
- [ ] `latency_ms` por comando con:
  - [ ] `count`
  - [ ] `avg_wait_ms`
  - [ ] `avg_exec_ms`
  - [ ] `stddev_wait_ms`
  - [ ] `stddev_exec_ms`
  - [ ] `p50`, `p95`, `p99`

### ✅ Comportamiento
- [ ] Las métricas se incrementan con más requests
- [ ] Cada comando tiene sus propias métricas
- [ ] Los valores de wait_ms son menores que exec_ms
- [ ] Los percentiles son crecientes (p50 < p95 < p99)

### ✅ Edge Cases
- [ ] Comandos que no existen no aparecen en métricas
- [ ] Después de reiniciar el servidor, las métricas empiezan en 0
- [ ] Múltiples requests simultáneas registran correctamente

## Troubleshooting

### No aparecen métricas de un comando
- Verifica que hayas hecho al menos 1 request exitoso (status 200)
- Espera unos segundos para que procese
- Verifica que el servidor está corriendo en el puerto correcto

### Los valores parecen incorrectos
- Verifica que estás usando el endpoint correcto
- Limpia el cache si estás usando un proxy
- Reinicia el servidor y prueba de nuevo

### No se puede conectar al servidor
- Verifica que `cargo run` está ejecutándose
- Verifica el puerto (por defecto 8080)
- Verifica que no hay firewall bloqueando

## Ejemplo de Salida Esperada

```json
{
  "queues": {
    "isprime": 0,
    "factor": 0,
    "status": 0
  },
  "workers": {
    "isprime": {
      "total": 4,
      "busy": 0
    },
    "factor": {
      "total": 4,
      "busy": 0
    }
  },
  "latency_ms": {
    "isprime": {
      "count": 10,
      "avg_wait_ms": 1.5,
      "avg_exec_ms": 12.3,
      "stddev_wait_ms": 0.8,
      "stddev_exec_ms": 2.1,
      "p50": 11,
      "p95": 16,
      "p99": 20
    },
    "factor": {
      "count": 3,
      "avg_wait_ms": 0.8,
      "avg_exec_ms": 8.5,
      "stddev_wait_ms": 0.3,
      "stddev_exec_ms": 1.2,
      "p50": 8,
      "p95": 10,
      "p99": 12
    }
  },
  "throughput": {
    "requests_per_second": 50
  },
  "requests": {
    "accepted": 13,
    "handled": 13,
    "errors": 0
  }
}
```

