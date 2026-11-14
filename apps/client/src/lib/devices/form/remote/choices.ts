import type { RdpResizeMethod, RdpSecurity } from '$lib/hub/generated/graphql';

/** The RDP security modes, as the choice names them; Standard RDP says it is weak (RA-020). */
export const RDP_SECURITY: readonly { value: RdpSecurity; label: string }[] = [
  { value: 'ANY', label: 'Any (the server chooses)' },
  { value: 'NLA', label: 'Network Level Authentication' },
  { value: 'NLA_EXT', label: 'Network Level Authentication, extended' },
  { value: 'TLS', label: 'TLS' },
  { value: 'VMCONNECT', label: 'Hyper-V virtual machine' },
  { value: 'RDP', label: 'Standard RDP (weak)' },
];

export const RDP_RESIZE: readonly { value: RdpResizeMethod; label: string }[] = [
  { value: 'DISPLAY_UPDATE', label: 'Display update' },
  { value: 'RECONNECT', label: 'Reconnect' },
];

/** The RDP on/off settings, each a switch that saves at once (D17). */
export const RDP_FLAGS = [
  { key: 'audioOut', label: 'Play sound from the device' },
  { key: 'microphone', label: 'Send the microphone' },
  { key: 'sharedDrive', label: 'Share a drive' },
  { key: 'ignoreCertificate', label: 'Ignore the server certificate' },
] as const;
