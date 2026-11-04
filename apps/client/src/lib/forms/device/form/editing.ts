import type { DeviceFormValues } from '../values';

type Writable<T> = { -readonly [Key in keyof T]: T[Key] extends readonly (infer Entry)[] ? Entry[] : Writable<T[Key]> };

/** The form values while the operator edits them; T1's `DeviceFormValues` is read-only. */
export type EditedValues = Writable<DeviceFormValues>;

/** A copy the operator can edit; the shared empty credential of `formValues` must never be written to. */
export function editable(values: DeviceFormValues): EditedValues {
  return { ...values, tags: [...values.tags], ssh: { ...values.ssh }, credential: { ...values.credential } };
}
