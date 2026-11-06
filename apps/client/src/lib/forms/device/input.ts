import type { CreateDeviceInput, SshCredentialInput, SshSettingsInput, UpdateDeviceInput } from '$lib/hub/generated/graphql';
import { normalizedValues, portToSend, type DeviceFormValues } from './values';

type NormalizedSsh = ReturnType<typeof normalizedValues>['ssh'];

/** The SSH settings as the hub input; shared with the connection test. */
export function sshInput(ssh: NormalizedSsh, initialPort?: string): SshSettingsInput {
  return { enabled: ssh.enabled, port: portToSend(ssh, initialPort), username: ssh.username, auth: ssh.auth };
}

/** Whether the SSH settings that would be sent differ from the saved ones. */
export function sameSsh(initial: NormalizedSsh, now: NormalizedSsh): boolean {
  const a = sshInput(initial);
  const b = sshInput(now, initial.port);
  return a.enabled === b.enabled && a.port === b.port && a.username === b.username && a.auth === b.auth;
}

function sameTags(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((tag, index) => tag === b[index]);
}

/** DEV-010: the create mutation input. The hub adds the default ICMP check (Section 6.2). */
export function createInput(values: DeviceFormValues, siteId: string): CreateDeviceInput {
  const now = normalizedValues(values);
  return {
    siteId,
    name: now.name,
    host: now.host,
    kind: now.kind,
    os: now.os,
    tags: [...now.tags],
    ...(now.ssh.enabled ? { ssh: sshInput(now.ssh) } : {}),
    addDefaultCheck: true,
  };
}

/** DEV-010: only the fields that differ from `initial`; an unchanged form gives `{}`. SSH goes whole, as the hub input needs all its parts. */
export function updateInput(initial: DeviceFormValues, values: DeviceFormValues): UpdateDeviceInput {
  const before = normalizedValues(initial);
  const now = normalizedValues(values);
  return {
    ...(now.name !== before.name ? { name: now.name } : {}),
    ...(now.host !== before.host ? { host: now.host } : {}),
    ...(now.kind !== before.kind ? { kind: now.kind } : {}),
    ...(now.os !== before.os ? { os: now.os } : {}),
    ...(sameTags(now.tags, before.tags) ? {} : { tags: [...now.tags] }),
    ...(sameSsh(before.ssh, now.ssh) ? {} : { ssh: sshInput(now.ssh, before.ssh.port) }),
  };
}

/**
 * DEV-012: the credential to send, or null when the chosen auth's fields are empty, so the stored one is kept (D-4).
 * Fields of the other auth choice are ignored.
 */
export function credentialInput(values: DeviceFormValues): SshCredentialInput | null {
  const { credential, ssh } = values;
  if (!ssh.enabled) return null;
  if (ssh.auth === 'PASSWORD') return credential.password === '' ? null : { password: credential.password };
  if (credential.privateKey.trim() === '') return null;
  return {
    privateKey: {
      key: credential.privateKey,
      ...(credential.passphrase === '' ? {} : { passphrase: credential.passphrase }),
    },
  };
}
