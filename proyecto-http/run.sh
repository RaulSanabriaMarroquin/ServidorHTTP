#!/bin/bash

# Script de configuración y ejecución del servidor HTTP
# Facilita la configuración y ejecución del servidor con diferentes perfiles

echo "🚀 CONFIGURACIÓN Y EJECUCIÓN DEL SERVIDOR HTTP"
echo "=============================================="

# Función para mostrar ayuda
show_help() {
    echo "Uso: $0 [OPCIÓN]"
    echo ""
    echo "Opciones:"
    echo "  dev          - Ejecutar en modo desarrollo"
    echo "  prod         - Ejecutar en modo producción"
    echo "  test         - Ejecutar pruebas"
    echo "  coverage     - Generar reporte de cobertura"
    echo "  load-test    - Ejecutar pruebas de carga"
    echo "  build        - Compilar el proyecto"
    echo "  clean        - Limpiar archivos de compilación"
    echo "  help         - Mostrar esta ayuda"
    echo ""
    echo "Variables de entorno soportadas:"
    echo "  PORT              - Puerto del servidor (default: 8080)"
    echo "  WORKERS_BASIC     - Workers para endpoints básicos (default: 2)"
    echo "  WORKERS_CPU       - Workers para CPU-bound (default: 4)"
    echo "  WORKERS_IO        - Workers para IO-bound (default: 4)"
    echo "  QUEUE_BASIC       - Profundidad cola básica (default: 64)"
    echo "  QUEUE_CPU         - Profundidad cola CPU (default: 128)"
    echo "  QUEUE_IO          - Profundidad cola IO (default: 128)"
    echo "  TIMEOUT_CPU_MS    - Timeout CPU en ms (default: 60000)"
    echo "  TIMEOUT_IO_MS      - Timeout IO en ms (default: 120000)"
    echo ""
    echo "Ejemplos:"
    echo "  $0 dev                    # Modo desarrollo"
    echo "  PORT=9090 $0 prod         # Puerto personalizado"
    echo "  WORKERS_CPU=8 $0 prod     # Más workers CPU"
}

# Función para verificar dependencias
check_dependencies() {
    echo "🔍 Verificando dependencias..."
    
    if ! command -v cargo &> /dev/null; then
        echo "❌ Error: Rust/Cargo no está instalado"
        echo "   Instala Rust desde: https://rustup.rs/"
        exit 1
    fi
    
    echo "✅ Rust/Cargo está disponible"
    
    # Verificar versión de Rust
    rust_version=$(cargo --version | cut -d' ' -f2)
    echo "📦 Versión de Rust: $rust_version"
}

# Función para compilar el proyecto
build_project() {
    echo ""
    echo "🔨 COMPILANDO PROYECTO"
    echo "====================="
    
    echo "Compilando en modo release..."
    cargo build --release
    
    if [ $? -eq 0 ]; then
        echo "✅ Compilación exitosa"
        echo "📁 Ejecutable: target/release/proyecto-http"
    else
        echo "❌ Error en la compilación"
        exit 1
    fi
}

# Función para limpiar archivos
clean_project() {
    echo ""
    echo "🧹 LIMPIANDO PROYECTO"
    echo "====================="
    
    echo "Limpiando archivos de compilación..."
    cargo clean
    
    echo "Limpiando archivos temporales..."
    rm -rf data/
    rm -rf coverage/
    rm -f *.log
    
    echo "✅ Limpieza completada"
}

# Función para ejecutar en modo desarrollo
run_dev() {
    echo ""
    echo "🛠️  MODO DESARROLLO"
    echo "=================="
    
    # Configuración para desarrollo
    export WORKERS_BASIC=${WORKERS_BASIC:-1}
    export WORKERS_CPU=${WORKERS_CPU:-2}
    export WORKERS_IO=${WORKERS_IO:-2}
    export QUEUE_BASIC=${QUEUE_BASIC:-16}
    export QUEUE_CPU=${QUEUE_CPU:-32}
    export QUEUE_IO=${QUEUE_IO:-32}
    export TIMEOUT_CPU_MS=${TIMEOUT_CPU_MS:-30000}
    export TIMEOUT_IO_MS=${TIMEOUT_IO_MS:-60000}
    
    echo "Configuración de desarrollo:"
    echo "  Puerto: ${PORT:-8080}"
    echo "  Workers básicos: $WORKERS_BASIC"
    echo "  Workers CPU: $WORKERS_CPU"
    echo "  Workers IO: $WORKERS_IO"
    echo "  Colas: $QUEUE_BASIC/$QUEUE_CPU/$QUEUE_IO"
    echo ""
    
    echo "Iniciando servidor en modo desarrollo..."
    cargo run
}

# Función para ejecutar en modo producción
run_prod() {
    echo ""
    echo "🚀 MODO PRODUCCIÓN"
    echo "=================="
    
    # Configuración para producción
    export WORKERS_BASIC=${WORKERS_BASIC:-4}
    export WORKERS_CPU=${WORKERS_CPU:-8}
    export WORKERS_IO=${WORKERS_IO:-6}
    export QUEUE_BASIC=${QUEUE_BASIC:-128}
    export QUEUE_CPU=${QUEUE_CPU:-256}
    export QUEUE_IO=${QUEUE_IO:-256}
    export TIMEOUT_CPU_MS=${TIMEOUT_CPU_MS:-120000}
    export TIMEOUT_IO_MS=${TIMEOUT_IO_MS:-300000}
    
    echo "Configuración de producción:"
    echo "  Puerto: ${PORT:-8080}"
    echo "  Workers básicos: $WORKERS_BASIC"
    echo "  Workers CPU: $WORKERS_CPU"
    echo "  Workers IO: $WORKERS_IO"
    echo "  Colas: $QUEUE_BASIC/$QUEUE_CPU/$QUEUE_IO"
    echo ""
    
    echo "Compilando en modo release..."
    cargo build --release
    
    if [ $? -eq 0 ]; then
        echo "✅ Compilación exitosa"
        echo "Iniciando servidor en modo producción..."
        ./target/release/proyecto-http
    else
        echo "❌ Error en la compilación"
        exit 1
    fi
}

# Función para ejecutar pruebas
run_tests() {
    echo ""
    echo "🧪 EJECUTANDO PRUEBAS"
    echo "===================="
    
    echo "Ejecutando pruebas unitarias..."
    cargo test --lib -- --nocapture
    
    if [ $? -eq 0 ]; then
        echo "✅ Pruebas unitarias pasaron"
    else
        echo "❌ Algunas pruebas fallaron"
        exit 1
    fi
    
    echo ""
    echo "Ejecutando pruebas de integración..."
    cargo test --test '*' -- --nocapture
    
    if [ $? -eq 0 ]; then
        echo "✅ Pruebas de integración pasaron"
    else
        echo "❌ Algunas pruebas de integración fallaron"
        exit 1
    fi
}

# Función para generar cobertura
run_coverage() {
    echo ""
    echo "📊 GENERANDO COBERTURA"
    echo "======================"
    
    if [ -f "coverage_test.sh" ]; then
        chmod +x coverage_test.sh
        ./coverage_test.sh
    else
        echo "❌ Script de cobertura no encontrado"
        echo "   Ejecuta: cargo install cargo-tarpaulin"
        echo "   Luego: cargo tarpaulin --out html"
    fi
}

# Función para ejecutar pruebas de carga
run_load_test() {
    echo ""
    echo "⚡ PRUEBAS DE CARGA"
    echo "=================="
    
    if [ -f "load_test.sh" ]; then
        chmod +x load_test.sh
        ./load_test.sh
    else
        echo "❌ Script de pruebas de carga no encontrado"
        echo "   Usa: ./test_all_endpoints.sh"
    fi
}

# Función para mostrar estado del proyecto
show_status() {
    echo ""
    echo "📊 ESTADO DEL PROYECTO"
    echo "======================"
    
    echo "Archivos principales:"
    ls -la src/ 2>/dev/null || echo "  ❌ Directorio src/ no encontrado"
    
    echo ""
    echo "Configuración:"
    if [ -f "Cargo.toml" ]; then
        echo "  ✅ Cargo.toml encontrado"
        echo "  📦 Dependencias:"
        grep -E "^\[dependencies\]|^[a-zA-Z]" Cargo.toml | head -10
    else
        echo "  ❌ Cargo.toml no encontrado"
    fi
    
    echo ""
    echo "Scripts disponibles:"
    ls -la *.sh 2>/dev/null || echo "  ❌ No hay scripts .sh"
    
    echo ""
    echo "Archivos de prueba:"
    ls -la test_* 2>/dev/null || echo "  ❌ No hay archivos de prueba"
}

# Función principal
main() {
    case "${1:-help}" in
        "dev")
            check_dependencies
            run_dev
            ;;
        "prod")
            check_dependencies
            run_prod
            ;;
        "test")
            check_dependencies
            run_tests
            ;;
        "coverage")
            check_dependencies
            run_coverage
            ;;
        "load-test")
            check_dependencies
            run_load_test
            ;;
        "build")
            check_dependencies
            build_project
            ;;
        "clean")
            clean_project
            ;;
        "status")
            show_status
            ;;
        "help"|"-h"|"--help")
            show_help
            ;;
        *)
            echo "❌ Opción desconocida: $1"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

# Verificar que estamos en el directorio correcto
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: No se encontró Cargo.toml"
    echo "   Ejecuta este script desde el directorio raíz del proyecto"
    exit 1
fi

# Ejecutar función principal
main "$@"
