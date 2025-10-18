/* 
Contratos para colas/pools por comando:

Priority (Low, Normal, High).

WorkItem (contendrá req, cmd, prio, enqueue_ts en sprints siguientes).

Handler (trait): handle(&self, WorkItem) -> Response. Cada endpoint implementará uno.

WorkQueue (trait): submit(...) -> Result<EnqueueOutcome, Backpressure>.

EnqueueOutcome::Immediate(Response): si se resuelve inline.

EnqueueOutcome::Enqueued{ job_id }: si va a cola (posible integración con Jobs).

Backpressure { retry_after_ms }: cola saturada ⇒ devolver 503.

DummyQueue: implementación “mock” que siempre devuelve 501 (sirve para compilar y probar el flujo).

Futuro:

Una WorkQueue real por comando con mpsc/Mutex+Condvar, N workers, y límites de cola.

Planificación FIFO por defecto + prioridades.
 */