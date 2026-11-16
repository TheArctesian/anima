-- The version of each device's agent (K2 0710-0729; R-118, ROL-051).
--
-- `agents.agent_version` is the `agentVersion` of the last accepted batch, null while the agent
-- has sent none: an agent older than R-118, or one that has not pushed since its token was made.
-- The hub writes it with the time of the last batch, so the two never disagree. The check is the
-- K8 rule's length; the batch check holds the rest of the rule (`agent-core`).
--
-- One nullable column with no default: no table rewrite, cheap on a large database (DB-010).

alter table metrics.agents add column agent_version text null
    check (octet_length(agent_version) between 1 and 64);
