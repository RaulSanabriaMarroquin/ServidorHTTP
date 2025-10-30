# Manual Técnico - Servidor HTTP/1.0

## Versión 0.1.0

**Proyecto:** Servidor HTTP Multihilo  
**Lenguaje:** Rust  
**Curso:** Principios de Sistemas Operativos  
**Institución:** Universidad Tecnológica de Costa Rica

---

## Índice

1. [Introducción](#introducción)
2. [Requisitos del Sistema](#requisitos-del-sistema)
3. [Instalación](#instalación)
4. [Arquitectura del Sistema](#arquitectura-del-sistema)
5. [Configuración](#configuración)
6. [API Reference](#api-reference)
7. [Sistema de Jobs](#sistema-de-jobs)
8. [Uso y Ejemplos](#uso-y-ejemplos)
9. [Testing y Cobertura](#testing-y-cobertura)
10. [Rendimiento y Optimización](#rendimiento-y-optimización)
11. [Monitoreo y Métricas](#monitoreo-y-métricas)
12. [Solución de Problemas](#solución-de-problemas)
13. [Desarrollo y Contribución](#desarrollo-y-contribución)

---

## Introducción

Este documento describimos la arquitectura, configuración que se implementó y uso del servidor HTTP/1.0 utilizando Rust para el proyecto del curso de SO. El servidor implementa conceptos importantes como concurrencia, sincronización, planificación de tareas y manejo de recursos compartidos.

### Objetivos del Proyecto

- Implementar un servidor HTTP/1.0 funcional capaz de manejar múltiples conexiones concurrentes
- Demostrar uso adecuado de threads y sincronización con Arc/Mutex
- Implementar pools de workers especializados por tipo de tarea
- Manejar tareas CPU-bound e IO-bound de forma eficiente
- Implementar un sistema de jobs con colas persistentes

### Características Principales

- Arquitectura multihilo thread-per-connection
- Pools de workers especializados por tipo de tarea
- Sistema de colas FIFO con backpressure
- Job Manager para tareas asíncronas largas
- Sincronización thread-safe con Arc/Mutex y canales mpsc
- Métricas en tiempo real de rendimiento y colas
- Configuración flexible mediante variables de entorno

---

## Requisitos del Sistema

### Requisitos Mínimos

- **Sistema Operativo:** Linux, macOS o Windows con WSL2
- **Rust:** Versión 1.70 o superior (edición 2021)
- **Memoria RAM:** Mínimo 512MB disponible
- **Espacio en Disco:** 500MB para compilación y datos de prueba

### Dependencias del Lenguaje

- `serde` y `serde_json` para serialización JSON
- `flate2` para compresión gzip
- `xz2` para compresión xz
- `sha2` y `hex` para cálculos de hash
- `num-bigint`, `num-integer`, `num-traits` para operaciones matemáticas avanzadas
- `lazy_static` para inicialización estática


---

## Instalación

### Clonar el Repositorio

```bash
git clone <url-del-repositorio>
cd ServidorHTTP/proyecto-http
```

### Compilar el Proyecto

#### Compilación en Modo Debug

Para desarrollo y depuración:

```bash
cargo build
```


### Ejecutar el Servidor

#### Ejecución Directa

```bash
cargo run
```

#### Ejecución con Configuración Personalizada

```bash
PORT=9090 WORKERS_CPU=8 cargo run
```

---

## Arquitectura del Sistema

### Visión General

El servidor implementa una arquitectura multihilo con los siguientes componentes principales:

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP Listener Thread                      │
│                  (Accept Loop - Main Thread)                 │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
              ┌─────────────────────┐
              │      Router          │
              │  (Route Matching)    │
              └──────────┬───────────┘
                         │
          ┌──────────────┼──────────────┐
          │              │              │
          ▼              ▼              ▼
    ┌──────────┐   ┌──────────┐   ┌──────────┐
    │ Basic    │   │   CPU    │   │    IO    │
    │ Worker   │   │  Worker  │   │  Worker  │
    │  Pool    │   │   Pool   │   │   Pool   │
    └────┬─────┘   └─────┬────┘   └────┬─────┘
         │               │              │
         ▼               ▼              ▼
    ┌──────────┐   ┌──────────┐   ┌──────────┐
    │  Queue   │   │  Queue   │   │  Queue   │
    │  (FIFO)  │   │  (FIFO)  │   │  (FIFO)  │
    └────┬─────┘   └─────┬────┘   └────┬─────┘
         │               │              │
         └───────────────┼───────────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │   Job Manager       │
              │ (Async Task Queue)  │
              └─────────────────────┘
```

### Componentes de Código

#### `src/main.rs`
Es el punto de entrada que se encarga de nicializar la configuración, router, estado compartido y arranca el listener HTTP.

#### `src/core.rs`
- Maneja conexiones HTTP entrantes
- Construcción de responses
- Estado compartido (AppState) envuelto en Arc

#### `src/router.rs`
- Tabla de enrutamiento
- Determina pool de destino y handler
- Enumera rutas disponibles

#### `src/workers.rs`
- Implementación de pools de workers
- Gestión de colas FIFO
- Sincronización thread-safe

#### `src/jobs.rs`
- Job Manager para tareas asíncronas
- Persistencia de timepo limitado de trabajos
- Estado de jobs (queued/running/done/error)

#### `src/config.rs`
- Carga de configuración desde variables de entorno
- Valores por defecto (en caso de que no se configuren)
- Múltiples alias para compatibilidad

#### `src/handlers/`
- `basic.rs`: Endpoints simples (status, timestamp, reverse, etc.)
- `cpu.rs`: Tareas intensivas en CPU (isprime, pi, mandelbrot)
- `io.rs`: Operaciones de I/O (sortfile, grep, compress)
- `jobs.rs`: Sistema de jobs (submit, status, result, cancel)

#### `src/metrics.rs`
- Recolección de métricas de rendimiento
- Cálculo de latencias p50/p95/p99
- Estadísticas de throughput

### Flujo de Procesamiento de Request

1. **Conexión**: Listener acepta nueva conexión TCP
2. **Parseo**: Se parsea request HTTP (método, path, query params)
3. **Enrutamiento**: Router determina handler y pool
4. **Encolado**: Request se encola en cola del pool correspondiente
5. **Procesamiento**: Worker libre toma request y ejecuta handler
6. **Respuesta**: Handler genera JSON de respuesta
7. **Envío**: Response se serializa a HTTP y se envía al cliente
8. **Cierre**: Conexión TCP se cierra

### Sincronización y Concurrencia

#### Arc<Mutex<T>>
Usado para estado compartido entre threads:
- Configuración
- Router (inmutable pero compartido)
- Métricas

#### Canales mpsc
Usados para comunicación entre threads:
- Envío de requests desde listener a workers
- Comunicación entre Job Manager y workers

#### Deadlocks
Diseño que evita deadlocks por ordenamiento de locks y timeouts en operaciones bloqueantes.

---

## Configuración

### Variables de Entorno

El servidor se configura mediante variables de entorno. Todos los valores son opcionales y tienen defaults apropiados.

#### Puerto del Servidor

```bash
PORT=8080              # Puerto principal
HTTP_PORT=8080         # Alias para PORT
```

#### Número de Workers

```bash
WORKERS_BASIC=2        # Workers para endpoints básicos
WORKERS_CPU=4          # Workers para tareas CPU-bound
WORKERS_IO=4           # Workers para tareas IO-bound

# Alias
BASIC_WORKERS=2
CPU_WORKERS=4
IO_WORKERS=4
```

#### Profundidad de Colas

```bash
QUEUE_BASIC=64         # Cola básica
QUEUE_CPU=128          # Cola CPU
QUEUE_IO=128           # Cola IO

# Alias
BASIC_QDEPTH=64
CPU_QDEPTH=128
IO_QDEPTH=128

# Otros alias
BASIC_QUEUE=64
CPU_QUEUE=128
IO_QUEUE=128
```

#### Timeouts

```bash
TIMEOUT_CPU_MS=60000   # Timeout para tareas CPU (60 segundos)
TIMEOUT_IO_MS=120000   # Timeout para tareas IO (120 segundos)
```

### Configuraciones Recomendadas

#### Desarrollo Local

```bash
export WORKERS_BASIC=1
export WORKERS_CPU=2
export WORKERS_IO=2
export QUEUE_BASIC=16
export QUEUE_CPU=32
export QUEUE_IO=32
cargo run
```

#### Producción Pequeña-Mediana

```bash
export PORT=8080
export WORKERS_BASIC=4
export WORKERS_CPU=8
export WORKERS_IO=6
export QUEUE_BASIC=128
export QUEUE_CPU=256
export QUEUE_IO=256
cargo run --release
```

#### Producción Alta Carga

```bash
export WORKERS_BASIC=8
export WORKERS_CPU=16
export WORKERS_IO=12
export QUEUE_BASIC=512
export QUEUE_CPU=1024
export QUEUE_IO=1024
export TIMEOUT_CPU_MS=120000
export TIMEOUT_IO_MS=300000
cargo run --release
```

### Carga de Configuración

La configuración se carga en orden de prioridad:

1. Variables de entorno con múltiples alias
2. Valores por defecto del sistema

El servidor imprime la configuración activa al iniciar:

```
[CONFIG] Config { port: 8080, workers_basic: 2, workers_cpu: 4, ... }
```

---

## API Reference

### Endpoints Básicos

#### GET /status
Retorna información del servidor y métricas.

**Ejemplo:**
```bash
curl http://localhost:8080/status
```

**Respuesta:**
```json
{
  "status": "ok",
  "port": 8080,
  "pid": 12345,
  "uptime_ms": 3600000,
  "metrics": {
    "accepted": 1000,
    "handled": 995
  },
  "queues": {
    "basic": {"pending": 0, "workers": 2},
    "cpu": {"pending": 3, "workers": 4},
    "io": {"pending": 1, "workers": 4}
  }
}
```

#### GET /timestamp
Retorna timestamp UNIX actual.

**Ejemplo:**
```bash
curl http://localhost:8080/timestamp
```

**Respuesta:**
```json
{
  "timestamp": 1704123456,
  "iso8601": "2024-01-01T12:00:00Z"
}
```

#### GET /help
Documentación de endpoints disponibles.

**Ejemplo:**
```bash
curl http://localhost:8080/help
```

#### GET /reverse?text=abc
Invierte una cadena de texto.

**Parámetros:**
- `text` (requerido): Cadena a invertir

**Ejemplo:**
```bash
curl "http://localhost:8080/reverse?text=hello"
```

**Respuesta:**
```json
{
  "original": "hello",
  "reversed": "olleh"
}
```

#### GET /toupper?text=abc
Convierte texto a mayúsculas.

**Parámetros:**
- `text` (requerido): Cadena a convertir

**Ejemplo:**
```bash
curl "http://localhost:8080/toupper?text=hello world"
```

**Respuesta:**
```json
{
  "original": "hello world",
  "uppercase": "HELLO WORLD"
}
```

#### GET /random?count=5&min=1&max=100
Genera números aleatorios.

**Parámetros:**
- `count` (requerido): Cantidad de números
- `min` (requerido): Valor mínimo
- `max` (requerido): Valor máximo

**Ejemplo:**
```bash
curl "http://localhost:8080/random?count=10&min=1&max=100"
```

**Respuesta:**
```json
{
  "count": 10,
  "min": 1,
  "max": 100,
  "numbers": [42, 15, 78, 91, 23, 56, 33, 89, 12, 67]
}
```

#### GET /hash?text=input
Calcula hash SHA-256 de texto.

**Parámetros:**
- `text` (requerido): Texto a hashear

**Ejemplo:**
```bash
curl "http://localhost:8080/hash?text=hello"
```

**Respuesta:**
```json
{
  "text": "hello",
  "hash": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
}
```

#### GET /fibonacci?n=10
Calcula número de Fibonacci.

**Parámetros:**
- `n` (requerido): Posición en secuencia

**Ejemplo:**
```bash
curl "http://localhost:8080/fibonacci?n=10"
```

**Respuesta:**
```json
{
  "n": 10,
  "fibonacci": 55,
  "elapsed_ms": 0
}
```

### Endpoints CPU-Bound

#### GET /isprime?n=97
Verifica si un número es primo.

**Parámetros:**
- `n` (requerido): Número a verificar

**Ejemplo:**
```bash
curl "http://localhost:8080/isprime?n=97"
```

**Respuesta:**
```json
{
  "n": 97,
  "is_prime": true,
  "method": "miller-rabin",
  "elapsed_ms": 12
}
```

#### GET /factor?n=360
Factorización en números primos.

**Parámetros:**
- `n` (requerido): Número a factorizar

**Ejemplo:**
```bash
curl "http://localhost:8080/factor?n=360"
```

**Respuesta:**
```json
{
  "n": 360,
  "factors": [[2, 3], [3, 2], [5, 1]],
  "elapsed_ms": 7
}
```

#### GET /pi?digits=100
Cálculo de π con algoritmo Spigot.

**Parámetros:**
- `digits` (requerido): Cantidad de dígitos (máx 1000 para performance)

**Ejemplo:**
```bash
curl "http://localhost:8080/pi?digits=100"
```

**Respuesta:**
```json
{
  "digits": 100,
  "pi": "3.14159265358979323...",
  "elapsed_ms": 250
}
```

#### GET /mandelbrot?width=200&height=200&max_iter=1000
Genera conjunto de Mandelbrot.

**Parámetros:**
- `width` (requerido): Ancho en píxeles
- `height` (requerido): Alto en píxeles
- `max_iter` (requerido): Iteraciones máximas

**Ejemplo:**
```bash
curl "http://localhost:8080/mandelbrot?width=100&height=100&max_iter=1000"
```

**Respuesta:**
```json
{
  "width": 100,
  "height": 100,
  "max_iter": 1000,
  "data": [[...], [...], ...],
  "elapsed_ms": 500
}
```

#### GET /matrixmul?size=100&seed=123
Multiplica dos matrices cuadradas.

**Parámetros:**
- `size` (requerido): Dimensiones de matrices
- `seed` (requerido): Semilla para números pseudoaleatorios

**Ejemplo:**
```bash
curl "http://localhost:8080/matrixmul?size=100&seed=123"
```

**Respuesta:**
```json
{
  "size": 100,
  "seed": 123,
  "result_sha256": "abc123...",
  "elapsed_ms": 1200
}
```

### Endpoints IO-Bound

#### GET /sortfile?name=file.txt&algo=merge
Ordena números en archivo.

**Parámetros:**
- `name` (requerido): Nombre del archivo en directorio `data/`
- `algo` (requerido): Algoritmo: `merge` o `quick`

**Ejemplo:**
```bash
curl "http://localhost:8080/sortfile?name=data.txt&algo=merge"
```

**Respuesta:**
```json
{
  "file": "data.txt",
  "algo": "merge",
  "sorted_file": "data.sorted.txt",
  "elements_sorted": 100000,
  "elapsed_ms": 1500
}
```

#### GET /wordcount?name=file.txt
Cuenta líneas, palabras y bytes (tipo wc).

**Parámetros:**
- `name` (requerido): Nombre del archivo

**Ejemplo:**
```bash
curl "http://localhost:8080/wordcount?name=data.txt"
```

**Respuesta:**
```json
{
  "file": "data.txt",
  "lines": 1000,
  "words": 5000,
  "bytes": 45678,
  "elapsed_ms": 250
}
```

#### GET /grep?name=file.txt&pattern=regex
Busca patrones en archivo.

**Parámetros:**
- `name` (requerido): Nombre del archivo
- `pattern` (requerido): Patrón regex

**Ejemplo:**
```bash
curl "http://localhost:8080/grep?name=data.txt&pattern=error"
```

**Respuesta:**
```json
{
  "file": "data.txt",
  "pattern": "error",
  "matches": 15,
  "sample_lines": ["line1", "line2", ...],
  "elapsed_ms": 180
}
```

#### GET /compress?name=file.txt&codec=gzip
Comprime archivo.

**Parámetros:**
- `name` (requerido): Nombre del archivo
- `codec` (requerido): Códec: `gzip` o `xz`

**Ejemplo:**
```bash
curl "http://localhost:8080/compress?name=data.txt&codec=gzip"
```

**Respuesta:**
```json
{
  "file": "data.txt",
  "codec": "gzip",
  "compressed_file": "data.txt.gz",
  "original_size": 1000000,
  "compressed_size": 250000,
  "ratio": 0.25,
  "elapsed_ms": 800
}
```

#### GET /hashfile?name=file.txt&algo=sha256
Calcula hash de archivo.

**Parámetros:**
- `name` (requerido): Nombre del archivo
- `algo` (requerido): Algoritmo: `sha256`

**Ejemplo:**
```bash
curl "http://localhost:8080/hashfile?name=data.txt&algo=sha256"
```

**Respuesta:**
```json
{
  "file": "data.txt",
  "algo": "sha256",
  "hash": "abc123...",
  "file_size": 1000000,
  "elapsed_ms": 120
}
```

#### GET /sleep?ms=1000
Sleep con límite de tiempo.

**Parámetros:**
- `ms` (requerido): Milisegundos a dormir

**Ejemplo:**
```bash
curl "http://localhost:8080/sleep?ms=500"
```

**Respuesta:**
```json
{
  "requested_ms": 500,
  "actual_ms": 500,
  "elapsed_ms": 500
}
```

#### GET /createfile?name=test.txt&content=hello&repeat=3
Crea archivo con contenido repetido.

**Parámetros:**
- `name` (requerido): Nombre del archivo
- `content` (requerido): Contenido
- `repeat` (requerido): Veces a repetir

**Ejemplo:**
```bash
curl "http://localhost:8080/createfile?name=test.txt&content=hello&repeat=3"
```

**Respuesta:**
```json
{
  "name": "test.txt",
  "content": "hello",
  "repeat": 3,
  "bytes_written": 15,
  "file_path": "data/test.txt"
}
```

#### GET /deletefile?name=test.txt
Elimina un archivo.

**Parámetros:**
- `name` (requerido): Nombre del archivo

**Ejemplo:**
```bash
curl "http://localhost:8080/deletefile?name=test.txt"
```

**Respuesta:**
```json
{
  "name": "test.txt",
  "deleted": true
}
```

#### GET /metrics
Métricas detalladas del sistema.

**Ejemplo:**
```bash
curl http://localhost:8080/metrics
```

**Respuesta:**
```json
{
  "queues": {
    "basic": {"pending": 0, "max_depth": 64, "workers": 2},
    "cpu": {"pending": 3, "max_depth": 128, "workers": 4},
    "io": {"pending": 1, "max_depth": 128, "workers": 4}
  },
  "workers": {
    "basic": {"total": 2, "busy": 0},
    "cpu": {"total": 4, "busy": 2},
    "io": {"total": 4, "busy": 1}
  },
  "latency_ms": {
    "basic": {"p50": 5, "p95": 15, "p99": 25},
    "cpu": {"p50": 50, "p95": 200, "p99": 500},
    "io": {"p50": 100, "p95": 1000, "p99": 5000}
  },
  "throughput": {
    "requests_per_second": 150
  }
}
```

### Códigos de Error HTTP

El servidor retorna códigos HTTP apropiados:

- `200 OK`: Request exitoso
- `400 Bad Request`: Parámetros inválidos o malformados
- `404 Not Found`: Ruta no existe
- `409 Conflict`: Operación no permitida (ej. eliminar archivo inexistente)
- `429 Too Many Requests`: Cola llena, demasiadas requests
- `500 Internal Server Error`: Error interno del servidor
- `503 Service Unavailable`: Servicio temporalmente no disponible (backpressure)

---

## Sistema de Jobs

Para tareas que pueden exceder timeout HTTP (5-15s), el servidor incluye un Job Manager con colas internas.

### Endpoints de Jobs

#### GET /jobs/submit?task=TASK&<params>
Encola trabajo y retorna job_id.

**Parámetros:**
- `task` (requerido): Nombre de tarea
- Parámetros específicos según tarea
- `prio` (opcional): Prioridad: `low`, `normal`, `high` (default: `normal`)

**Ejemplo:**
```bash
curl "http://localhost:8080/jobs/submit?task=isprime&n=999983&prio=high"
```

**Respuesta:**
```json
{
  "job_id": "job-1704123456-abc123",
  "status": "queued",
  "task": "isprime",
  "estimated_time_ms": 5000
}
```

#### GET /jobs/status?id=JOBID
Estado de un job.

**Parámetros:**
- `id` (requerido): ID del job

**Ejemplo:**
```bash
curl "http://localhost:8080/jobs/status?id=job-1704123456-abc123"
```

**Respuesta:**
```json
{
  "job_id": "job-1704123456-abc123",
  "status": "running",
  "progress": 42,
  "eta_ms": 3800
}
```

Posibles estados:
- `queued`: Esperando en cola
- `running`: Ejecutándose
- `done`: Completado
- `error`: Error durante ejecución
- `canceled`: Cancelado por usuario

#### GET /jobs/result?id=JOBID
Resultado de un job.

**Parámetros:**
- `id` (requerido): ID del job

**Ejemplo:**
```bash
curl "http://localhost:8080/jobs/result?id=job-1704123456-abc123"
```

**Respuesta (si status=done):**
```json
{
  "job_id": "job-1704123456-abc123",
  "status": "done",
  "result": {
    "n": 999983,
    "is_prime": true,
    "method": "miller-rabin",
    "elapsed_ms": 4800
  }
}
```

**Respuesta (si status=error):**
```json
{
  "job_id": "job-1704123456-abc123",
  "status": "error",
  "error": "Timeout exceeded"
}
```

**Respuesta (si aún no está listo):**
```json
{
  "job_id": "job-1704123456-abc123",
  "status": "running",
  "error": "Job not yet completed"
}
```

#### GET /jobs/cancel?id=JOBID
Cancela un job (si está en cola o running).

**Parámetros:**
- `id` (requerido): ID del job

**Ejemplo:**
```bash
curl "http://localhost:8080/jobs/cancel?id=job-1704123456-abc123"
```

**Respuesta:**
```json
{
  "job_id": "job-1704123456-abc123",
  "status": "canceled",
  "message": "Job canceled successfully"
}
```

### Semántica de Jobs

#### Planificación
- FIFO por defecto
- Soporta prioridades: `low`, `normal`, `high`
- Límite de concurrencia por tipo de comando

#### Backpressure
Si la cola excede umbral:
- Retorna `503 Service Unavailable`
- Incluye `retry_after_ms`

#### Timeouts
- Tiempo máx por trabajo: 60s CPU, 120s IO
- Al exceder: status `error` con mensaje `timeout`

#### Persistencia
- Metadatos de jobs sobreviven a graceful restart
- Journal simple en archivo temporal

#### Tareas Soportadas
Todas las rutas CPU/IO soportan ejecución vía job:
- Tasks CPU: `isprime`, `factor`, `pi`, `mandelbrot`, `matrixmul`
- Tasks IO: `sortfile`, `wordcount`, `grep`, `compress`, `hashfile`

---

## Uso y Ejemplos

### Ejemplo 1: Health Check Básico

```bash
# Verificar que el servidor está corriendo
curl http://localhost:8080/status | jq

# Ver timestamp actual
curl http://localhost:8080/timestamp

# Ver ayuda
curl http://localhost:8080/help
```

### Ejemplo 2: Tareas CPU-Bound

```bash
# Verificar si número es primo
curl "http://localhost:8080/isprime?n=97"

# Factorizar número
curl "http://localhost:8080/factor?n=360"

# Calcular pi con 50 dígitos
curl "http://localhost:8080/pi?digits=50"

# Multiplicar matrices 200x200
curl "http://localhost:8080/matrixmul?size=200&seed=123"
```

### Ejemplo 3: Operaciones de Archivos

```bash
# Crear archivo de prueba
curl "http://localhost:8080/createfile?name=test.txt&content=hello&repeat=1000"

# Contar palabras en archivo
curl "http://localhost:8080/wordcount?name=test.txt"

# Buscar patrones
curl "http://localhost:8080/grep?name=test.txt&pattern=hello"

# Comprimir archivo
curl "http://localhost:8080/compress?name=test.txt&codec=gzip"

# Calcular hash
curl "http://localhost:8080/hashfile?name=test.txt&algo=sha256"
```

### Ejemplo 4: Jobs Asíncronos

```bash
# Submit job para verificar primalidad
JOB_ID=$(curl -s "http://localhost:8080/jobs/submit?task=isprime&n=999983" | jq -r .job_id)

# Esperar un momento
sleep 2

# Verificar estado
curl "http://localhost:8080/jobs/status?id=$JOB_ID"

# Esperar a que termine
for i in {1..10}; do
  STATUS=$(curl -s "http://localhost:8080/jobs/status?id=$JOB_ID" | jq -r .status)
  if [ "$STATUS" = "done" ]; then
    break
  fi
  sleep 1
done

# Obtener resultado
curl "http://localhost:8080/jobs/result?id=$JOB_ID"
```

### Ejemplo 5: Script de Pruebas

Crear archivo `test_server.sh`:

```bash
#!/bin/bash
BASE_URL="http://localhost:8080"

echo "=== Health Check ==="
curl -s "$BASE_URL/status" | jq

echo -e "\n=== Timestamp ==="
curl -s "$BASE_URL/timestamp" | jq

echo -e "\n=== Reverse ==="
curl -s "$BASE_URL/reverse?text=hello" | jq

echo -e "\n=== Random ==="
curl -s "$BASE_URL/random?count=5&min=1&max=100" | jq

echo -e "\n=== Is Prime ==="
curl -s "$BASE_URL/isprime?n=97" | jq

echo -e "\n=== Metrics ==="
curl -s "$BASE_URL/metrics" | jq
```

Ejecutar:
```bash
chmod +x test_server.sh
./test_server.sh
```

---

## Testing y Cobertura

### Pruebas Unitarias

#### Ejecutar Todas las Pruebas

```bash
cargo test
```

#### Output Detallado

```bash
cargo test -- --nocapture
```

#### Pruebas Específicas

```bash
# Solo pruebas de fibonacci
cargo test fibonacci

# Solo pruebas de isprime
cargo test isprime

# Solo pruebas de handlers
cargo test handlers
```

#### Pruebas con Verificación de Documentación

```bash
cargo test --doc
```

### Pruebas de Endpoints

El proyecto incluye scripts para pruebas de endpoints:

```bash
# Script de pruebas de todos los endpoints
chmod +x test_all_endpoints.sh
./test_all_endpoints.sh

# Guardar output
./test_all_endpoints.sh > test_output.txt
```

### Cobertura de Código

#### Instalar cargo-tarpaulin

```bash
cargo install cargo-tarpaulin
```

#### Generar Reporte de Cobertura

```bash
# Cobertura completa
cargo tarpaulin

# Con output detallado
cargo tarpaulin --out html

# Guardar reporte
cargo tarpaulin --out xml
```

El reporte HTML se genera en `tarpaulin-report.html`.

#### Meta de Cobertura

- Mínimo requerido: 90%
- Objetivo: >95%
- Todas las funciones públicas deben tener tests

### Pruebas de Concurrencia

#### Pruebas de Carga con Apache Bench

```bash
# Instalar ab
# Ubuntu/Debian
sudo apt-get install apache2-utils

# macOS
brew install ab

# Prueba básica
ab -n 1000 -c 10 http://localhost:8080/status

# Prueba más intensa
ab -n 10000 -c 100 http://localhost:8080/timestamp
```

#### Pruebas de Carga con wrk

```bash
# Instalar wrk
# Ubuntu/Debian
sudo apt-get install wrk

# macOS
brew install wrk

# Prueba básica
wrk -t4 -c100 -d30s http://localhost:8080/status

# Prueba intensiva CPU
wrk -t8 -c200 -d60s http://localhost:8080/isprime?n=999983

# Con pipeline
wrk -t8 -c200 -d60s -s pipeline.lua http://localhost:8080/timestamp
```

#### Script de Pruebas de Concurrencia

```bash
# Ejecutar múltiples requests simultáneos
for i in {1..50}; do
  curl "http://localhost:8080/isprime?n=97" &
done
wait
```

### Pruebas de Stress

```bash
# Prueba de stress general
./load_test.sh

# Prueba específica de endpoints
./coverage_test.sh
```

---

## Rendimiento y Optimización

### Benchmarks Típicos

#### Básicos
- Requests/sec: 200-500
- Latencia p50: 2-10ms
- Latencia p95: 10-30ms
- Latencia p99: 30-100ms

#### CPU-Bound
- Requests/sec: 10-100
- Latencia p50: 50-500ms
- Latencia p95: 200-5000ms
- Latencia p99: 500-15000ms

#### IO-Bound
- Requests/sec: 50-200
- Latencia p50: 100-1000ms
- Latencia p95: 500-5000ms
- Latencia p99: 2000-15000ms

### Factores de Rendimiento

#### Número de Workers
- Pocos workers: latencia alta pero baja contención
- Muchos workers: mayor throughput pero contención de recursos
- Ajustar según carga

#### Profundidad de Colas
- Colas pequeñas: rechazo temprano (503)
- Colas grandes: mejor throughput pero uso de memoria

#### Modo de Compilación
- Debug: ~5x más lento que release
- Release: optimizaciones completas, LTO habilitado

### Perfilamiento

#### Rust Profiling

```bash
# Instalar perf (Linux)
sudo apt-get install linux-perf

# Compilar con debug info
RUSTFLAGS="-g" cargo build --release

# Perfilar
perf record -g ./target/release/proyecto-http &

# Generar reporte
perf report
```

#### Análisis de Memoria

```bash
# Usar valgrind (Linux)
valgrind --tool=massif ./target/release/proyecto-http

# Ver reporte
ms_print massif.out.PID
```

#### Monitoreo de Recursos

```bash
# Monitorear CPU y memoria
htop -p $(pgrep proyecto-http)

# Con top
top -p $(pgrep proyecto-http)

# Ver threads
ps -eLf | grep proyecto-http
```

### Optimizaciones Aplicadas

- Compilación release con `opt-level = 3`
- Link-time optimization (LTO)
- Pool de workers especializados
- Colas FIFO eficientes
- Serialización JSON optimizada
- Evitar clones innecesarios
- Reuso de buffers

---

## Monitoreo y Métricas

### Endpoint /metrics

Proporciona métricas detalladas en tiempo real:

```bash
curl http://localhost:8080/metrics | jq
```

**Estructura de métricas:**

```json
{
  "queues": {
    "pool_name": {
      "pending": 0,
      "max_depth": 128,
      "workers": 4
    }
  },
  "workers": {
    "pool_name": {
      "total": 4,
      "busy": 2
    }
  },
  "latency_ms": {
    "pool_name": {
      "p50": 50,
      "p95": 200,
      "p99": 500
    }
  },
  "throughput": {
    "requests_per_second": 150
  }
}
```

### Interpretación de Métricas

#### Queues
- `pending`: Número de requests en cola
- `max_depth`: Tamaño máximo de cola
- Alto `pending`: posible cuello de botella

#### Workers
- `total`: Total de workers en pool
- `busy`: Workers ocupados
- `busy == total`: posible necesidad de más workers

#### Latency
- `p50`: Mediana de latencia
- `p95`: Latencia del percentil 95
- `p99`: Latencia del percentil 99
- Aumento en percentiles superiores: posibles trabajos lentos

#### Throughput
- `requests_per_second`: Requests procesadas por segundo
- Comparar con latencia para identificar eficiencia

### Endpoint /status

Información básica del servidor:

```bash
curl http://localhost:8080/status | jq
```

Incluye:
- Estado del servidor
- Puerto y PID
- Uptime
- Métricas básicas
- Estado de colas
- Configuración activa

### Alertas Recomendadas

- `queue.pending > queue.max_depth * 0.8`: Alta carga en cola
- `latency_p99 > timeout`: Timeouts frecuentes
- `throughput == 0`: Servidor bloqueado
- `workers.busy == workers.total` por >60s: Saturación

---

## Solución de Problemas

### Problemas Comunes

#### Puerto en Uso

**Error:**
```
[FATAL] listener terminó con error: Address already in use
```

**Solución:**
```bash
# Cambiar puerto
export PORT=9090
cargo run

# O matar proceso en puerto 8080
# Linux
sudo lsof -ti:8080 | xargs kill

# Windows
netstat -ano | findstr :8080
taskkill /PID <PID> /F
```

#### Workers Bloqueados

**Síntoma:** Request se queda colgado, timeout

**Diagnóstico:**
```bash
curl http://localhost:8080/metrics | jq
# Verificar pending
```

**Solución:**
```bash
# Aumentar workers
export WORKERS_CPU=8
export WORKERS_IO=6
# Reiniciar servidor
```

#### Jobs No Se Procesan

**Síntoma:** Jobs quedan en `queued` indefinidamente

**Diagnóstico:**
```bash
curl "http://localhost:8080/jobs/status?id=job-XXX" | jq
curl http://localhost:8080/metrics | jq
```

**Solución:**
- Verificar que hay workers activos
- Verificar timeouts no son muy cortos
- Verificar que jobs no exceden capacidad

#### Errores de Memoria

**Síntoma:** Servidor crashea con out of memory

**Diagnóstico:**
```bash
# Monitorear memoria
ps aux | grep proyecto-http
```

**Solución:**
- Reducir número de workers
- Reducir profundidad de colas
- Revisar manejo de archivos grandes

#### Archivos No Encontrados

**Error:**
```
[ERROR] File not found: data/archivo.txt
```

**Solución:**
```bash
# Verificar que archivo existe
ls -lh data/

# Crear directorio si no existe
mkdir -p data
```

### Logs y Debugging

#### Habilitar Logs Verbosos

```bash
# Compilar con debug info
RUSTFLAGS="-g" cargo build

# Ejecutar con stdout visible
cargo run 2>&1 | tee server.log
```

#### Ver Llamadas del Sistema

```bash
# Linux con strace
strace -o trace.log ./target/release/proyecto-http

# Ver llamadas de red
strace -e trace=network ./target/release/proyecto-http
```

### Recuperación de Errores

#### Reinicio Graceful

```bash
# Enviar SIGTERM
kill -TERM $(pgrep proyecto-http)

# Servidor cierra conexiones activas y termina
```

#### Reinicio Fuerza

```bash
# Enviar SIGKILL
kill -9 $(pgrep proyecto-http)
```

---

## Desarrollo y Contribución

### Estructura del Código

```
proyecto-http/
├── src/
│   ├── main.rs          # Punto de entrada
│   ├── core.rs          # Núcleo HTTP, manejo conexiones
│   ├── router.rs        # Enrutamiento
│   ├── workers.rs       # Pools de workers
│   ├── jobs.rs          # Job Manager
│   ├── config.rs        # Configuración
│   ├── metrics.rs       # Métricas
│   └── handlers/         # Implementación endpoints
│       ├── mod.rs
│       ├── basic.rs     # Endpoints básicos
│       ├── cpu.rs       # CPU-bound
│       ├── io.rs        # IO-bound
│       └── jobs.rs      # Sistema de jobs
├── data/                # Archivos de datos
├── Cargo.toml          # Dependencias
├── Cargo.lock          # Lock de versiones
└── README.md           # Documentación general
```

### Convenciones de Código

- Usar `rustfmt` para formateo
- Documentar funciones públicas con `///`
- Usar `clippy` para linting
- Nombrar con snake_case
- Types con PascalCase

#### Formateo

```bash
cargo fmt
```

#### Linting

```bash
cargo clippy
cargo clippy -- -W clippy::all
```

### Agregar Nuevos Endpoints

1. Definir handler en `handlers/`

```rust
pub fn nuevo_endpoint(params: &HashMap<String, String>) -> Result<serde_json::Value, String> {
    // Implementación
}
```

2. Agregar ruta en `router.rs`

```rust
"/nuevo" => Route::Basic(handlers::basic::nuevo_endpoint),
```

3. Agregar tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_nuevo_endpoint() {
        // Test
    }
}
```

### Testing

- Tests unitarios por función
- Tests de integración por endpoint
- Coverage mínimo 90%
- Mocking cuando sea necesario

### Compromisos (Commits)

Usar mensajes descriptivos:

```
feat: agregar endpoint /nuevo
fix: corregir timeout en jobs
refactor: optimizar pools de workers
docs: actualizar documentación de métricas
test: agregar tests para sortfile
```

### Pull Requests

- Describir cambios
- Incluir tests
- Verificar que pasa `cargo test`
- Verificar que pasa `cargo clippy`
- Actualizar documentación si aplica

---

## Referencias

### Documentación Rust

- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [The Cargo Book](https://doc.rust-lang.org/cargo/)

### Sistemas Operativos

- Stevens, W. R. (2004). Advanced Programming in the UNIX Environment
- Tanenbaum, A. S., & Bos, H. (2014). Modern Operating Systems

### HTTP/1.0

- RFC 1945: Hypertext Transfer Protocol – HTTP/1.0

### Herramientas

- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin)
- [wrk](https://github.com/wg/wrk)
- [Apache Bench](https://httpd.apache.org/docs/2.4/programs/ab.html)

---

**Versión del Documento:** 1.0  
**Última Actualización:** Enero 2024  
**Autor:** Equipo de Desarrollo  
**Curso:** Principios de Sistemas Operativos  
**Institución:** Universidad Tecnológica de Costa Rica
