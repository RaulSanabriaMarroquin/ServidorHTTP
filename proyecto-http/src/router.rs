/* 
Tabla de enrutamiento path → Command (enum con todas las rutas del proyecto).

route_path(&str) -> Option<Command>: mapea la URL a un comando (e.g. /isprime → Command::IsPrime).

handle_early(&Request) -> Option<Response>: placeholder para Sprint 0; responde 200 con JSON { "routed": ... } si el path es válido, o 404 si no existe.

Futuro:

Validación temprana de parámetros (tipos, rangos).

Elección de prioridad (low/normal/high) desde query o header.

Decisión “ejecución directa vs encolar” (best effort). */