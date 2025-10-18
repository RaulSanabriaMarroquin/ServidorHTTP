/*Punto de entrada del binario.

Crea la configuración (Config), el router (Router) y el estado compartido (Shared = Arc<AppState>).

Llama al bucle del listener HTTP (http_listen_loop) que se queda aceptando conexiones.

En Sprint 1, acá agregaremos lectura de CLI/env y (posible) inicialización de pools por comando y JobManager. */