import type { FieldError } from '$lib/hub/hub';
import { emptyRemoteSecrets } from '$lib/devices/form/remote/values';
import type { DeviceFormValues } from '$lib/devices/form/values';

/**
 * What one change sends with it (D17). A device field goes alone; a protocol and the transfer flags go whole, as the hub
 * input takes them whole (`updateInput`), so a protocol's other fields ride along at their shown values.
 */
export type SettingsGroup = 'name' | 'host' | 'kind' | 'os' | 'tags' | 'macAddress' | 'metricsSource' | 'ssh' | 'rdp' | 'vnc' | 'transfer';

/** A setting the tab saves on its own, by its field path: `name`, `ssh.port`, `transfer.copy`. */
export type SettingKey = SettingsGroup | `${'ssh' | 'rdp' | 'vnc' | 'transfer'}.${string}`;

/** The group a setting saves with. */
export function groupOf(key: SettingKey): SettingsGroup {
  return key.split('.')[0] as SettingsGroup;
}

/** Whether an error's field path belongs to `group`; the SSH credential's errors belong to SSH. */
export function inGroup(field: string, group: SettingsGroup): boolean {
  if (group === 'ssh' && field.startsWith('credential.')) return true;
  return field === group || field.startsWith(`${group}.`);
}

/** The saved values with `group` as shown, and no secrets: what one change sends (D17, D-2). */
export function withGroup(saved: DeviceFormValues, shown: DeviceFormValues, group: SettingsGroup): DeviceFormValues {
  return {
    ...saved,
    [group]: structuredClone(shown[group]),
    credential: { password: '', privateKey: '', passphrase: '' },
    remoteSecrets: emptyRemoteSecrets(),
  };
}

/** Whether `group` is shown as it is saved. */
export function sameGroup(a: DeviceFormValues, b: DeviceFormValues, group: SettingsGroup): boolean {
  return JSON.stringify(a[group]) === JSON.stringify(b[group]);
}

/** The value of a setting's field path in `values`. */
export function valueAt(values: DeviceFormValues, key: SettingKey): unknown {
  const [group, field] = key.split('.') as [SettingsGroup, string | undefined];
  const top: unknown = values[group];
  return field === undefined ? top : (top as Record<string, unknown>)[field];
}

/** Puts `value` at a setting's field path of the values being edited. */
export function setValueAt(values: DeviceFormValues, key: SettingKey, value: unknown): void {
  const [group, field] = key.split('.') as [SettingsGroup, string | undefined];
  if (field === undefined) (values as unknown as Record<string, unknown>)[group] = structuredClone(value);
  else (values[group] as unknown as Record<string, unknown>)[field] = structuredClone(value);
}

/** The errors with those of `group` replaced by `next`. */
export function replaceGroup(errors: readonly FieldError[], group: SettingsGroup, next: readonly FieldError[]): FieldError[] {
  return [...errors.filter((error) => !inGroup(error.field, group)), ...next];
}
