-- The stale mark of an agent (K2 0710-0729; MET-018, R-098 a).
--
-- `agents.stale_at` is when the hub marked the device's agent stale, null while it is not. The
-- stale watch sets it once no batch was accepted for 30 seconds, and the next accepted batch
-- clears it; each change is one statement that also says whether it changed anything, so each
-- way writes one event. An agent that has never pushed (`last_batch_at` null) is never marked:
-- it never became stale.
--
-- One nullable column with no default: no table rewrite, cheap on a large database (DB-010).

alter table metrics.agents add column stale_at timestamptz null;
