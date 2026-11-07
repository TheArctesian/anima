declare const fingerprintBrand: unique symbol;

/** The SHA-256 of a leaf certificate as the shell and the hub write it: 64 lower-case hex digits (D-5, H-4). */
export type Fingerprint = string & { readonly [fingerprintBrand]: true };

const FORM = /^[0-9a-f]{64}$/;

/** `value` as a `Fingerprint`, or `null` when it is not exactly 64 lower-case hex digits. Nothing is normalised. */
export function fingerprintFrom(value: unknown): Fingerprint | null {
  return typeof value === 'string' && FORM.test(value) ? (value as Fingerprint) : null;
}

/** How the operator reads a fingerprint: colon-separated pairs, upper case, as SEC-007 host keys (D-5). */
export function displayFingerprint(fingerprint: Fingerprint): string {
  return (fingerprint.match(/../g) ?? []).join(':').toUpperCase();
}
