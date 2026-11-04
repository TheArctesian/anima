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

/** Tags trimmed, empty ones dropped, duplicates removed; the first occurrence wins. */
export function normalizeTags(tags: readonly string[]): string[] {
  return [...new Set(tags.map((tag) => tag.trim()).filter((tag) => tag !== ''))];
}

/** The values as they will be sent: trimmed text, bare host, clean tags. Secrets are not part of it. */
export function normalizedValues(values: DeviceFormValues): Omit<DeviceFormValues, 'credential'> {
  return {
    name: values.name.trim(),
    host: normalizeHost(values.host),
    kind: values.kind.trim(),
    os: values.os,
    tags: normalizeTags(values.tags),
    ssh: { ...values.ssh, port: values.ssh.port.trim(), username: values.ssh.username.trim() },
  };
}
