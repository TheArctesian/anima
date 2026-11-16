import type { AccessWarning } from '$lib/devices/api';

const RDP_WARNINGS = {
  ignoreCertificate: 'Ignoring the certificate means an impostor would not be detected.',
  weakSecurity: 'Standard RDP security is weak: the sign-in is poorly protected.',
} as const;

/** RA-020: the warnings the RDP choices earn, as the operator is about to save them. */
export function rdpWarnings(rdp: { readonly ignoreCertificate: boolean; readonly security: string }): string[] {
  return [
    ...(rdp.ignoreCertificate ? [RDP_WARNINGS.ignoreCertificate] : []),
    ...(rdp.security === 'RDP' ? [RDP_WARNINGS.weakSecurity] : []),
  ];
}

/** RA-020: the sentence for a warning the hub reports on the saved VNC settings, or null for one that is not about VNC. */
export function vncWarning(warnings: readonly AccessWarning[]): string | null {
  return warnings.includes('UNENCRYPTED_VNC')
    ? 'VNC is not encrypted. Use it only over the VPN; prefer RDP or SSH.'
    : null;
}

/** The date part of an RFC 3339 expiry, for reading. */
export function validUntil(notAfter: string): string {
  return notAfter.slice(0, 10);
}
