import type { AcceptHostKeyMutation, DeviceQuery, OperatingSystem, SitesQuery, SshAuth } from '$lib/hub/generated/graphql';
import type { ThisMachine } from '$lib/desktop/thisMachine';
import { normalizeHost } from './host';
import {
  defaultRemoteValues,
  emptyRemoteSecrets,
  normalizedRemote,
  remoteValues,
  type RemoteFields,
  type RemoteSecrets,
  type RemoteValues,
} from './remote/values';

/** The part of a hub `Device` the form edits. The K1 fields are optional until the devices client has them (D-3). */
export type DeviceFields = Pick<NonNullable<DeviceQuery['device']>, 'name' | 'host' | 'kind' | 'os' | 'tags'> & {
  readonly ssh: Pick<NonNullable<DeviceQuery['device']>['ssh'], 'enabled' | 'port' | 'username' | 'auth'>;
} & RemoteFields;

/** A site the Add device form can place the device in. */
export type SiteOption = SitesQuery['sites'][number];

/** A host key the hub saw, as the connection test and the host key modal show it. */
export type HostKey = AcceptHostKeyMutation['acceptHostKey'];

/** What the operator typed. Text fields stay text so that "abc" in the port box is reportable. */
export interface DeviceFormValues extends RemoteValues {
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
  /** Never filled from the hub (DEV-011). */
  readonly remoteSecrets: RemoteSecrets;
}

function emptyCredential(): DeviceFormValues['credential'] {
  return { password: '', privateKey: '', passphrase: '' };
}

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
      credential: emptyCredential(),
      remoteSecrets: emptyRemoteSecrets(),
      ...defaultRemoteValues(),
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
    credential: emptyCredential(),
    remoteSecrets: emptyRemoteSecrets(),
    ...remoteValues(device),
  };
}

/**
 * The Add device values for the computer the app runs on (R-058, R-060): everything the shell could learn,
 * so the operator only types the password. The kind is `workstation`: the form's word for a computer a person
 * works at, which covers a laptop and a desktop alike (the shell cannot tell them apart).
 */
export function thisMachineValues(machine: ThisMachine): DeviceFormValues {
  return {
    name: machine.name,
    host: machine.vpnAddress,
    kind: 'workstation',
    os: machine.os,
    tags: [],
    ssh: { enabled: machine.sshListening, port: '22', username: machine.user, auth: 'PASSWORD' },
    credential: emptyCredential(),
    remoteSecrets: emptyRemoteSecrets(),
    ...defaultRemoteValues(),
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
export function normalizeTags(tags: readonly string[]): string[] {
  return [...new Set(tags.map((tag) => tag.trim()).filter((tag) => tag !== ''))];
}

/** The values as they will be sent: trimmed text, bare host, clean tags. Secrets are not part of it. */
export function normalizedValues(values: DeviceFormValues): Omit<DeviceFormValues, 'credential' | 'remoteSecrets'> {
  return {
    name: values.name.trim(),
    host: normalizeHost(values.host),
    kind: values.kind,
    os: values.os,
    tags: normalizeTags(values.tags),
    ssh: { ...values.ssh, port: values.ssh.port.trim(), username: values.ssh.username.trim() },
    ...normalizedRemote(values),
  };
}
