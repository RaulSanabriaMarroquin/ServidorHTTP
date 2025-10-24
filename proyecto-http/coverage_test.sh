#!/bin/bash

# Script de pruebas de cobertura para el servidor HTTP
# Genera reportes de cobertura de código y ejecuta todas las pruebas

echo "🧪 INICIANDO PRUEBAS DE COBERTURA"
echo "================================="

# Función para verificar dependencias
check_dependencies() {
    echo "🔍 Verificando dependencias..."
    
    if ! command -v cargo &> /dev/null; then
        echo "❌ Error: cargo no está instalado"
        exit 1
    fi
    
    if ! command -v cargo-tarpaulin &> /dev/null; then
        echo "📦 Instalando cargo-tarpaulin para cobertura..."
        cargo install cargo-tarpaulin
        if [ $? -ne 0 ]; then
            echo "❌ Error: No se pudo instalar cargo-tarpaulin"
            exit 1
        fi
    fi
    
    echo "✅ Dependencias verificadas"
}

# Función para ejecutar pruebas unitarias
run_unit_tests() {
    echo ""
    echo "🧪 EJECUTANDO PRUEBAS UNITARIAS"
    echo "==============================="
    
    echo "Ejecutando todas las pruebas..."
    cargo test --lib -- --nocapture
    
    if [ $? -eq 0 ]; then
        echo "✅ Todas las pruebas unitarias pasaron"
    else
        echo "❌ Algunas pruebas unitarias fallaron"
        return 1
    fi
}

# Función para ejecutar pruebas de integración
run_integration_tests() {
    echo ""
    echo "🔗 EJECUTANDO PRUEBAS DE INTEGRACIÓN"
    echo "===================================="
    
    echo "Ejecutando pruebas de integración..."
    cargo test --test '*' -- --nocapture
    
    if [ $? -eq 0 ]; then
        echo "✅ Todas las pruebas de integración pasaron"
    else
        echo "❌ Algunas pruebas de integración fallaron"
        return 1
    fi
}

# Función para generar reporte de cobertura
generate_coverage_report() {
    echo ""
    echo "📊 GENERANDO REPORTE DE COBERTURA"
    echo "================================="
    
    echo "Generando reporte de cobertura con tarpaulin..."
    
    # Generar reporte HTML
    cargo tarpaulin --out html --output-dir coverage/
    
    if [ $? -eq 0 ]; then
        echo "✅ Reporte de cobertura generado en coverage/"
        echo "   Abre coverage/tarpaulin-report.html en tu navegador"
    else
        echo "❌ Error generando reporte de cobertura"
        return 1
    fi
    
    # Generar reporte de texto
    echo "Generando reporte de texto..."
    cargo tarpaulin --out stdout
    
    if [ $? -eq 0 ]; then
        echo "✅ Reporte de texto generado"
    else
        echo "❌ Error generando reporte de texto"
        return 1
    fi
}

# Función para generar reporte de cobertura detallado
generate_detailed_coverage() {
    echo ""
    echo "📈 GENERANDO COBERTURA DETALLADA"
    echo "================================"
    
    echo "Generando reporte detallado con información de líneas..."
    
    # Generar reporte con información de líneas no cubiertas
    cargo tarpaulin --out html --output-dir coverage/ --verbose
    
    if [ $? -eq 0 ]; then
        echo "✅ Reporte detallado generado"
    else
        echo "❌ Error generando reporte detallado"
        return 1
    fi
}

# Función para analizar cobertura
analyze_coverage() {
    echo ""
    echo "🔍 ANALIZANDO COBERTURA"
    echo "======================="
    
    # Extraer porcentaje de cobertura del reporte
    if [ -f "coverage/tarpaulin-report.html" ]; then
        echo "Analizando reporte de cobertura..."
        
        # Buscar porcentaje de cobertura en el HTML
        coverage_percent=$(grep -o 'coverage: [0-9]*\.[0-9]*%' coverage/tarpaulin-report.html | head -1 | grep -o '[0-9]*\.[0-9]*')
        
        if [ -n "$coverage_percent" ]; then
            echo "📊 Cobertura actual: ${coverage_percent}%"
            
            # Verificar si cumple con el requisito del 90%
            if (( $(echo "$coverage_percent >= 90.0" | bc -l) )); then
                echo "✅ Cumple con el requisito de cobertura ≥90%"
            else
                echo "⚠️  No cumple con el requisito de cobertura ≥90%"
                echo "   Se requiere mejorar la cobertura en $((90 - ${coverage_percent%.*}))%"
            fi
        else
            echo "❌ No se pudo extraer el porcentaje de cobertura"
        fi
    else
        echo "❌ No se encontró el reporte de cobertura"
    fi
}

# Función para generar reporte de líneas no cubiertas
uncovered_lines_report() {
    echo ""
    echo "📝 REPORTE DE LÍNEAS NO CUBIERTAS"
    echo "================================="
    
    echo "Generando reporte de líneas no cubiertas..."
    
    # Generar reporte que muestre líneas no cubiertas
    cargo tarpaulin --out stdout --verbose | grep -E "(uncovered|not covered)" || echo "No se encontraron líneas no cubiertas específicas"
    
    echo "✅ Reporte de líneas no cubiertas generado"
}

# Función para ejecutar pruebas de rendimiento
performance_tests() {
    echo ""
    echo "⚡ PRUEBAS DE RENDIMIENTO"
    echo "========================"
    
    echo "Ejecutando pruebas de rendimiento..."
    
    # Medir tiempo de compilación
    echo "Midiendo tiempo de compilación..."
    start_time=$(date +%s%3N)
    cargo build --release
    end_time=$(date +%s%3N)
    compile_time=$((end_time - start_time))
    echo "⏱️  Tiempo de compilación: ${compile_time}ms"
    
    # Medir tiempo de ejecución de pruebas
    echo "Midiendo tiempo de ejecución de pruebas..."
    start_time=$(date +%s%3N)
    cargo test --lib
    end_time=$(date +%s%3N)
    test_time=$((end_time - start_time))
    echo "⏱️  Tiempo de pruebas: ${test_time}ms"
    
    echo "✅ Pruebas de rendimiento completadas"
}

# Función para generar reporte final
generate_final_report() {
    echo ""
    echo "📋 REPORTE FINAL DE COBERTURA"
    echo "============================="
    
    report_file="coverage/coverage_report.txt"
    mkdir -p coverage/
    
    {
        echo "REPORTE DE COBERTURA DE CÓDIGO"
        echo "=============================="
        echo "Fecha: $(date)"
        echo "Proyecto: Servidor HTTP/1.0"
        echo ""
        echo "RESUMEN:"
        echo "--------"
        
        if [ -f "coverage/tarpaulin-report.html" ]; then
            coverage_percent=$(grep -o 'coverage: [0-9]*\.[0-9]*%' coverage/tarpaulin-report.html | head -1 | grep -o '[0-9]*\.[0-9]*')
            echo "Cobertura total: ${coverage_percent}%"
            
            if (( $(echo "$coverage_percent >= 90.0" | bc -l) )); then
                echo "Estado: ✅ CUMPLE (≥90%)"
            else
                echo "Estado: ❌ NO CUMPLE (<90%)"
            fi
        else
            echo "Cobertura total: No disponible"
            echo "Estado: ❌ ERROR"
        fi
        
        echo ""
        echo "ARCHIVOS PRINCIPALES:"
        echo "--------------------"
        echo "- src/main.rs: Punto de entrada"
        echo "- src/core.rs: Núcleo HTTP"
        echo "- src/router.rs: Enrutamiento"
        echo "- src/workers.rs: Pools de workers"
        echo "- src/jobs.rs: Sistema de jobs"
        echo "- src/handlers/: Implementación de endpoints"
        echo "- src/config.rs: Configuración"
        echo "- src/metrics.rs: Métricas"
        
        echo ""
        echo "RECOMENDACIONES:"
        echo "---------------"
        if [ -n "$coverage_percent" ] && (( $(echo "$coverage_percent < 90.0" | bc -l) )); then
            echo "- Agregar más pruebas unitarias"
            echo "- Cubrir casos de error"
            echo "- Probar funciones auxiliares"
            echo "- Agregar pruebas de integración"
        else
            echo "- Mantener cobertura alta"
            echo "- Agregar pruebas para nuevos features"
            echo "- Revisar casos edge"
        fi
        
        echo ""
        echo "ARCHIVOS GENERADOS:"
        echo "------------------"
        echo "- coverage/tarpaulin-report.html: Reporte HTML"
        echo "- coverage/coverage_report.txt: Este reporte"
        
    } > "$report_file"
    
    echo "✅ Reporte final generado en $report_file"
    cat "$report_file"
}

# Función para limpiar archivos temporales
cleanup() {
    echo ""
    echo "🧹 LIMPIEZA"
    echo "==========="
    
    echo "Limpiando archivos temporales..."
    
    # Limpiar archivos de prueba generados
    rm -rf target/debug/deps/proyecto_http-*
    rm -f Cargo.lock.bak
    
    echo "✅ Limpieza completada"
}

# Función principal
main() {
    check_dependencies
    
    # Ejecutar todas las pruebas
    run_unit_tests
    unit_test_result=$?
    
    run_integration_tests
    integration_test_result=$?
    
    # Generar reportes de cobertura
    generate_coverage_report
    coverage_result=$?
    
    generate_detailed_coverage
    detailed_coverage_result=$?
    
    # Analizar resultados
    analyze_coverage
    uncovered_lines_report
    performance_tests
    generate_final_report
    
    cleanup
    
    echo ""
    echo "🎉 PRUEBAS DE COBERTURA COMPLETADAS"
    echo "==================================="
    
    # Resumen final
    echo "📊 RESUMEN FINAL:"
    if [ $unit_test_result -eq 0 ]; then
        echo "✅ Pruebas unitarias: PASARON"
    else
        echo "❌ Pruebas unitarias: FALLARON"
    fi
    
    if [ $integration_test_result -eq 0 ]; then
        echo "✅ Pruebas de integración: PASARON"
    else
        echo "❌ Pruebas de integración: FALLARON"
    fi
    
    if [ $coverage_result -eq 0 ]; then
        echo "✅ Reporte de cobertura: GENERADO"
    else
        echo "❌ Reporte de cobertura: ERROR"
    fi
    
    echo ""
    echo "📁 ARCHIVOS GENERADOS:"
    echo "- coverage/tarpaulin-report.html (reporte HTML)"
    echo "- coverage/coverage_report.txt (reporte de texto)"
    echo ""
    echo "🔍 Para ver el reporte HTML:"
    echo "   open coverage/tarpaulin-report.html"
    echo "   (o abre el archivo en tu navegador)"
}

# Verificar que estamos en el directorio correcto
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: No se encontró Cargo.toml"
    echo "   Ejecuta este script desde el directorio raíz del proyecto"
    exit 1
fi

# Ejecutar función principal
main
