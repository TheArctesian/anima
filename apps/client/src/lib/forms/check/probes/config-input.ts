import type { PendingCheckConfigInput, PendingCheckType, SnmpConfigInput } from '$lib/forms/hub-pending';
import { toNumber } from '../model/range';
import type { ProbeValues } from './values';

function snmpInput({ snmp }: ProbeValues): SnmpConfigInput {
  const expected = snmp.expected.trim();
  // An empty secret is sent as is: the hub then keeps the stored one (SEC-005). Only the secrets of the chosen version go.
  const secrets =
    snmp.version === 'V2C'
      ? { community: snmp.community }
      : { username: snmp.username.trim(), authPassword: snmp.authPassword, privacyPassword: snmp.privacyPassword };
  return {
    port: toNumber(snmp.port),
    version: snmp.version,
    oid: snmp.oid.trim(),
    ...(expected === '' ? {} : { expected }),
    ...secrets,
    timeoutMs: toNumber(snmp.timeoutMs),
  };
}

/** The `@oneOf` config of a Phase 2 check type. */
export function probeInput(values: ProbeValues, type: PendingCheckType): PendingCheckConfigInput {
  switch (type) {
    case 'DOCKER':
      return { docker: { container: values.docker.container.trim(), timeoutMs: toNumber(values.docker.timeoutMs) } };
    case 'SYSTEMD':
      return { systemd: { unit: values.systemd.unit.trim(), timeoutMs: toNumber(values.systemd.timeoutMs) } };
    case 'SNMP':
      return { snmp: snmpInput(values) };
    case 'TAILSCALE_PING': {
      const target = values.tailscalePing.target.trim();
      return { tailscalePing: { ...(target === '' ? {} : { target }), timeoutMs: toNumber(values.tailscalePing.timeoutMs) } };
    }
  }
}
