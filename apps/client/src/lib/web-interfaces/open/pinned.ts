import { displayFingerprint, fingerprintFrom } from '$lib/web-interfaces/shell/fingerprint';

/**
 * The pinned fingerprint of an interface as the operator reads it, or `null` when there is nothing to show: a pin
 * belongs to `https` interfaces only, and a value that is not a SHA-256 fingerprint is not shown (WEB-011, D-5).
 */
export function pinnedFingerprintToShow(webInterface: { readonly url: string; readonly pinnedFingerprintSha256?: string | null }): string | null {
  if (URL.parse(webInterface.url)?.protocol !== 'https:') return null;
  const pin = fingerprintFrom(webInterface.pinnedFingerprintSha256);
  return pin === null ? null : displayFingerprint(pin);
}
