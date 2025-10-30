# Servidor HTTP/1.0 - Proyecto 1

**Servidor HTTP/1.0 multihilo implementado en Rust para el curso Principios de Sistemas Operativos**

## Descripción

Este proyecto implementa un servidor HTTP/1.0 completo que demuestra conceptos fundamentales de sistemas operativos:
- **Concurrencia**: Múltiples clientes simultáneos
- **Sincronización**: Arc<Mutex<...>> y canales mpsc
- **Planificación**: Pools de workers por tipo de tarea
- **Gestión de recursos**: Colas con backpressure

### Componentes Principales

- **`core.rs`**: Núcleo HTTP, manejo de conexiones
- **`router.rs`**: Enrutamiento de requests
- **`workers.rs`**: Pools de workers con colas
- **`jobs.rs`**: Sistema de trabajos asíncronos
- **`handlers/`**: Implementación de endpoints
- **`config.rs`**: Configuración del servidor
- **`metrics.rs`**: Métricas y observabilidad

## Instalación y Ejecución

### Compilación
```bash
# Clonar el repositorio
git clone <https://github.com/RaulSanabriaMarroquin/ServidorHTTP.git>
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

## Pruebas

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

##  Autores

- **David A.** 
- **Raúl M.** 