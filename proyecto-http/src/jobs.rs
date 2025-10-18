/* 
Contratos del Job Manager:

JobStatus (Queued, Running, Done, Error, Canceled).

JobStatusView (status, progress, ETA).

JobStore (trait): submit/status/result/cancel/recover_from_journal.

DummyJobStore: stub (compila y no hace nada).

Futuro:

Implementación con cola interna, journal (data/jobs.journal en JSON-lines) para persistencia efímera y polling por /jobs/*.
 */