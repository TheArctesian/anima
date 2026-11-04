import type { DeviceFields, OperatingSystem, SshAuth } from './hub-types';
import { normalizeHost } from './host';

/** What the operator typed. Text fields stay text so that "abc" in the port box is reportable. */
export interface DeviceFormValues {
  readonly name: string;
  readonly host: string;
  readonly kind: string;
  readonly os: OperatingSystem;
  readonly tags: readonly string[];
  readonly ssh: {
    readonly enabled: boolean;
    readonly port: string;
    readonly username: string;
    readonly auth: SshAuth;
  };
  /** Never filled from the hub (D-4); empty means "keep the stored credential". */
  readonly credential: {
    readonly password: string;
    readonly privateKey: string;
    readonly passphrase: string;
  };
}

const EMPTY_CREDENTIAL = { password: '', privateKey: '', passphrase: '' } as const;

/** The form values for a new device (`null`) or for editing `device`. */
export function formValues(device: DeviceFields | null): DeviceFormValues {
  if (device === null) {
    return {
      name: '',
      host: '',
      kind: 'server',
      os: 'OTHER',
      tags: [],
      ssh: { enabled: false, port: '22', username: '', auth: 'PASSWORD' },
      credential: EMPTY_CREDENTIAL,
    };
  }
  return {
    name: device.name,
    host: device.host,
    kind: device.kind,
    os: device.os,
    tags: [...device.tags],
    ssh: {
      enabled: device.ssh.enabled,
      port: String(device.ssh.port),
      username: device.ssh.username,
      auth: device.ssh.auth,
    },
    credential: EMPTY_CREDENTIAL,
  };
}

const MAX_PORT = 65535;
const DEFAULT_SSH_PORT = 22;

/** DEV-010: whether `text` is an integer port from 1 to 65535. */
export function isValidPort(text: string): boolean {
  return /^\d{1,5}$/.test(text) && Number(text) >= 1 && Number(text) <= MAX_PORT;
}

/**
 * The port to send. With SSH off, an unusable port box falls back to the last valid port
 * of `initial`, else 22, so the hub never receives garbage for a setting that is not in use.
 */
export function portToSend(ssh: { readonly enabled: boolean; readonly port: string }, initialPort: string | undefined): number {
  const typed = ssh.port.trim();
  if (isValidPort(typed) || ssh.enabled) return Number(typed);
  const earlier = initialPort?.trim() ?? '';
  return isValidPort(earlier) ? Number(earlier) : DEFAULT_SSH_PORT;
}

/** Tags trimmed, empty ones dropped, duplicates removed; the first occurrence wins. */
function normalizeTags(tags: readonly string[]): string[] {
  return [...new Set(tags.map((tag) => tag.trim()).filter((tag) => tag !== ''))];
}

/** The values as they will be sent: trimmed text, bare host, clean tags. Secrets are not part of it. */
export function normalizedValues(values: DeviceFormValues): Omit<DeviceFormValues, 'credential'> {
  return {
    name: values.name.trim(),
    host: normalizeHost(values.host),
    kind: values.kind,
    os: values.os,
    tags: normalizeTags(values.tags),
    ssh: { ...values.ssh, port: values.ssh.port.trim(), username: values.ssh.username.trim() },
  };
}
