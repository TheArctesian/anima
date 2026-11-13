-- Device credentials are bound to the device host they were entered for (K2 range 0210-0299;
-- SEC-011, security review 2026-11-13).
--
-- A secret sealed from now on holds its host inside the sealed value, under the AEAD context
-- `credentials.secrets.host:<device_id>:<kind>`, so no column is needed for it. A secret
-- sealed before has no host inside, and SQL cannot open it to add one. For such a row this
-- column keeps the host its device had when this migration ran: the assumption is that the
-- secret was entered for that host. The hub reads the column only to open a row sealed under
-- the old context `credentials.secrets:<device_id>:<kind>`; a row sealed with its host does not
-- open under that context, so setting the column on it does not rebind it. Storing the secret
-- again replaces the row and leaves the column null.
--
-- Privileges are unchanged: the new column is covered by the table grants of 0001.

alter table credentials.secrets add column legacy_host text null;

update credentials.secrets s set legacy_host = d.host
from devices.devices d
where d.id = s.device_id;
