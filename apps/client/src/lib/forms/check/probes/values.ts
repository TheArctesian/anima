import type { PendingCheckConfig, SnmpVersion } from '$lib/forms/hub-pending';

/** What the operator typed for the Docker, systemd, SNMP and Tailscale ping checks (CHK-009). */
export interface ProbeValues {
  docker: { container: string; timeoutMs: string };
  systemd: { unit: string; timeoutMs: string };
  snmp: {
    port: string;
    version: SnmpVersion;
    oid: string;
    expected: string;
    /** Never filled from the hub; empty means "keep the stored one" (SEC-005). */
    community: string;
    username: string;
    /** Never filled from the hub; empty means "keep the stored one" (SEC-005). */
    authPassword: string;
    /** Never filled from the hub; empty means "keep the stored one" (SEC-005). */
    privacyPassword: string;
    /** The version the hub holds secrets for; null for a new check. */
    storedVersion: SnmpVersion | null;
    timeoutMs: string;
  };
  tailscalePing: { target: string; timeoutMs: string };
}

/** The K1 defaults of the four checks. */
export function blankProbes(): ProbeValues {
  return {
    docker: { container: '', timeoutMs: '10000' },
    systemd: { unit: '', timeoutMs: '10000' },
    snmp: {
      port: '161',
      version: 'V2C',
      oid: '',
      expected: '',
      community: '',
      username: '',
      authPassword: '',
      privacyPassword: '',
      storedVersion: null,
      timeoutMs: '5000',
    },
    tailscalePing: { target: '', timeoutMs: '5000' },
  };
}

/** `values` with the settings read from a saved config; no secret is filled in. */
export function withProbeConfig<V extends ProbeValues>(values: V, config: PendingCheckConfig): V {
  switch (config.__typename) {
    case 'DockerConfig':
      return { ...values, docker: { container: config.container, timeoutMs: String(config.timeoutMs) } };
    case 'SystemdConfig':
      return { ...values, systemd: { unit: config.unit, timeoutMs: String(config.timeoutMs) } };
    case 'SnmpConfig':
      return {
        ...values,
        snmp: {
          ...values.snmp,
          port: String(config.port),
          version: config.version,
          oid: config.oid,
          expected: config.expected ?? '',
          username: config.username ?? '',
          storedVersion: config.version,
          timeoutMs: String(config.timeoutMs),
        },
      };
    case 'TailscalePingConfig':
      return { ...values, tailscalePing: { target: config.target ?? '', timeoutMs: String(config.timeoutMs) } };
  }
}
