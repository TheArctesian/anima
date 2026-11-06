/** A device's ID as the hub writes it: a UUID (K7). Only `parseDeviceId` makes one. */
export type DeviceId = string & { readonly __brand: 'DeviceId' };

const UUID = /^[\da-f]{8}-[\da-f]{4}-[\da-f]{4}-[\da-f]{4}-[\da-f]{12}$/i;

/** The route parameter as a `DeviceId`, or null when it is not one, so no request is sent for it (D-14). */
export function parseDeviceId(text: string): DeviceId | null {
  return UUID.test(text) ? (text as DeviceId) : null;
}
