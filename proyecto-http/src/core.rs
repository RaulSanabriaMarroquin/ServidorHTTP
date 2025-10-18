/* Núcleo HTTP/1.0 (mínimo viable):

AppState (config, métricas, router, start_at).

Shared = Arc<AppState> para compartir estado entre hilos.

Tipos Request (id, método, path, query, headers, raw) y Response (status, headers, body).

http_listen_loop: TcpListener.bind, incoming() y manejo de una conexión:

Lee hasta 8 KB, parsea la línea GET /ruta?x=1 HTTP/1.0 y headers.

Separa path y query.

Genera X-Request-Id simple (req-1, req-2, …).

Llama a router.handle_early(&req); si no hay ruta, responde 404; si hay, placeholder 200 indicando qué comando sería; si nada implementado, 501.

Serializa HTTP/1.0 (status line, headers —incluye X-Request-Id y Content-Length—, body).

Futuro (Sprint 1+):

spawn de hilos por conexión (o thread-pool).

Conversión Request → WorkItem y envío a WorkQueue por comando.

Manejo de timeouts a nivel de socket.
 */