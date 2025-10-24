# 🚀 Servidor HTTP/1.0 - Proyecto PSO1

**Servidor HTTP/1.0 multihilo implementado en Rust para el curso Principios de Sistemas Operativos**

## 📋 Descripción

Este proyecto implementa un servidor HTTP/1.0 completo que demuestra conceptos fundamentales de sistemas operativos:
- **Concurrencia**: Múltiples clientes simultáneos
- **Sincronización**: Arc<Mutex<...>> y canales mpsc
- **Planificación**: Pools de workers por tipo de tarea
- **Gestión de recursos**: Colas con backpressure

## 🏗️ Arquitectura

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   HTTP Listener │───▶│     Router      │───▶│   Worker Pools  │
│   (main thread) │    │   (routing)      │    │  basic/cpu/io   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │   Job Manager   │
                       │ (async tasks)   │
                       └─────────────────┘
```

### Componentes Principales

- **`core.rs`**: Núcleo HTTP, manejo de conexiones
- **`router.rs`**: Enrutamiento de requests
- **`workers.rs`**: Pools de workers con colas
- **`jobs.rs`**: Sistema de trabajos asíncronos
- **`handlers/`**: Implementación de endpoints
- **`config.rs`**: Configuración del servidor
- **`metrics.rs`**: Métricas y observabilidad

## 🚀 Instalación y Ejecución

### Prerrequisitos
- Rust 1.70+ (edición 2021)
- Sistema operativo Unix-like (Linux/macOS/WSL)

### Compilación
```bash
# Clonar el repositorio
git clone <tu-repo>
cd proyecto-http

# Compilar en modo release
cargo build --release

# Ejecutar
cargo run
```

### Configuración

El servidor se puede configurar mediante variables de entorno:

```bash
# Puerto del servidor (default: 8080)
export PORT=9090

# Número de workers por pool
export WORKERS_BASIC=4
export WORKERS_CPU=8
export WORKERS_IO=6

# Profundidad de colas
export QUEUE_BASIC=128
export QUEUE_CPU=256
export QUEUE_IO=256

# Timeouts (milisegundos)
export TIMEOUT_CPU_MS=120000
export TIMEOUT_IO_MS=300000

# Ejecutar con configuración personalizada
cargo run
```

## 📡 Endpoints Disponibles

### Endpoints Básicos
- `GET /status` - Estado del servidor y métricas
- `GET /timestamp` - Timestamp UNIX actual
- `GET /help` - Documentación de endpoints
- `GET /reverse?text=abc` - Invertir texto
- `GET /toupper?text=abc` - Convertir a mayúsculas
- `GET /random?count=5&min=1&max=100` - Números aleatorios
- `GET /hash?text=input` - Calcular hash
- `GET /fibonacci?n=10` - Número de Fibonacci
- `GET /isprime?n=97` - Verificar primalidad

### Endpoints CPU-Bound
- `GET /factor?n=360` - Factorización en primos
- `GET /pi?digits=100` - Cálculo de π (algoritmo Spigot)
- `GET /mandelbrot?width=100&height=100&max_iter=1000` - Conjunto de Mandelbrot
- `GET /matrixmul?size=100&seed=123` - Multiplicación de matrices

### Endpoints IO-Bound
- `GET /sortfile?name=file.txt&algo=merge` - Ordenar archivo
- `GET /wordcount?name=file.txt` - Contar palabras (tipo wc)
- `GET /grep?name=file.txt&pattern=regex` - Buscar patrones
- `GET /compress?name=file.txt&codec=gzip` - Comprimir archivo
- `GET /hashfile?name=file.txt&algo=sha256` - Hash de archivo

### Sistema de Jobs
- `GET /jobs/submit?task=isprime&n=97&prio=high` - Encolar trabajo
- `GET /jobs/status?id=job-123` - Estado del trabajo
- `GET /jobs/result?id=job-123` - Resultado del trabajo
- `GET /jobs/cancel?id=job-123` - Cancelar trabajo

### Archivos y Utilidades
- `GET /createfile?name=test.txt&content=hello&repeat=3` - Crear archivo
- `GET /deletefile?name=test.txt` - Eliminar archivo
- `GET /simulate?seconds=2&task=cpu_intensive` - Simular trabajo
- `GET /sleep?ms=1000` - Sleep con límite
- `GET /loadtest?tasks=10&sleep=100` - Prueba de carga
- `GET /metrics` - Métricas detalladas del sistema

## 🧪 Pruebas

### Pruebas Unitarias
```bash
# Ejecutar todas las pruebas
cargo test

# Pruebas con output detallado
cargo test -- --nocapture

# Pruebas específicas
cargo test fibonacci
cargo test isprime
```

### Pruebas de Endpoints
```bash
# Ejecutar script de pruebas completo
chmod +x test_all_endpoints.sh
./test_all_endpoints.sh
```

### Pruebas de Carga
```bash
# Instalar herramientas de carga
cargo install cargo-tarpaulin  # Para cobertura
# Instalar wrk para pruebas de carga
# sudo apt-get install wrk  # Ubuntu/Debian
# brew install wrk          # macOS

# Prueba de carga básica
wrk -t12 -c400 -d30s http://localhost:8080/status
```

## 📊 Métricas y Monitoreo

### Endpoint `/metrics`
Retorna métricas detalladas en formato JSON:
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
  "throughput": {"requests_per_second": 150}
}
```

### Endpoint `/status`
Información básica del servidor:
```json
{
  "status": "ok",
  "port": 8080,
  "pid": 12345,
  "uptime_ms": 3600000,
  "metrics": {"accepted": 1000, "handled": 995},
  "queues": [...],
  "config": {...}
}
```

## 🔧 Configuración Avanzada

### Variables de Entorno Completas
```bash
# Puerto y binding
PORT=8080                    # Puerto del servidor
HTTP_PORT=8080              # Alias para PORT

# Workers por pool
WORKERS_BASIC=2             # Workers para endpoints básicos
WORKERS_CPU=4               # Workers para CPU-bound
WORKERS_IO=4                # Workers para IO-bound
BASIC_WORKERS=2             # Alias
CPU_WORKERS=4               # Alias
IO_WORKERS=4                # Alias

# Profundidad de colas
QUEUE_BASIC=64              # Cola básica
QUEUE_CPU=128               # Cola CPU
QUEUE_IO=128                # Cola IO
BASIC_QDEPTH=64             # Alias
CPU_QDEPTH=128              # Alias
IO_QDEPTH=128               # Alias

# Timeouts
TIMEOUT_CPU_MS=60000        # Timeout CPU (60s)
TIMEOUT_IO_MS=120000        # Timeout IO (120s)
```

### Ejemplos de Configuración

#### Servidor de Alto Rendimiento
```bash
export WORKERS_BASIC=8
export WORKERS_CPU=16
export WORKERS_IO=12
export QUEUE_BASIC=512
export QUEUE_CPU=1024
export QUEUE_IO=1024
cargo run --release
```

#### Servidor de Desarrollo
```bash
export WORKERS_BASIC=1
export WORKERS_CPU=2
export WORKERS_IO=2
export QUEUE_BASIC=16
export QUEUE_CPU=32
export QUEUE_IO=32
cargo run
```

## 🏗️ Arquitectura Técnica

### Concurrencia
- **Thread-per-connection**: Cada conexión se maneja en un hilo separado
- **Worker Pools**: Pools especializados por tipo de tarea
- **Arc<Mutex<...>>**: Sincronización thread-safe para estado compartido
- **Canales mpsc**: Comunicación entre threads

### Planificación
- **FIFO por defecto**: Trabajos se procesan en orden de llegada
- **Prioridades**: Sistema de jobs soporta low/normal/high
- **Backpressure**: Límites de cola con error 503

### Persistencia
- **Journal de jobs**: Persistencia efímera en archivo
- **Recuperación**: Jobs sobreviven a restarts graceful
- **Cleanup**: Limpieza automática de jobs antiguos

## 🐛 Troubleshooting

### Problemas Comunes

#### Puerto en Uso
```bash
# Error: Address already in use
# Solución: Cambiar puerto
export PORT=9090
cargo run
```

#### Workers Bloqueados
```bash
# Verificar métricas
curl http://localhost:8080/metrics

# Si hay muchos pending, aumentar workers
export WORKERS_CPU=8
```

#### Jobs No Se Procesan
```bash
# Verificar estado del job
curl http://localhost:8080/jobs/status?id=job-123

# Verificar logs del servidor
cargo run -- --verbose
```

## 📈 Rendimiento

### Benchmarks Típicos
- **Requests/sec**: 100-500 (dependiendo del endpoint)
- **Latencia p50**: 5-50ms (básicos vs CPU-bound)
- **Latencia p95**: 15-500ms
- **Memoria**: ~50MB base + ~1MB por worker

### Optimizaciones
- Usar `cargo run --release` para producción
- Ajustar número de workers según carga
- Monitorear métricas en `/metrics`
- Usar pools apropiados para cada tipo de tarea

## 🤝 Contribución

1. Fork el proyecto
2. Crea una rama para tu feature (`git checkout -b feature/AmazingFeature`)
3. Commit tus cambios (`git commit -m 'Add some AmazingFeature'`)
4. Push a la rama (`git push origin feature/AmazingFeature`)
5. Abre un Pull Request

## 📄 Licencia

Este proyecto es parte del curso Principios de Sistemas Operativos de la Universidad Tecnológica de Costa Rica.

## 👥 Autores

- **Tu Nombre** - *Desarrollo inicial* - [TuGitHub](https://github.com/tuusuario)

## 🙏 Agradecimientos

- Profesor Kenneth Obando Rodríguez
- Curso Principios de Sistemas Operativos
- Comunidad Rust por las excelentes herramientas

---

**Nota**: Este servidor está diseñado para fines educativos y demostrar conceptos de sistemas operativos. No está optimizado para uso en producción sin modificaciones adicionales.
