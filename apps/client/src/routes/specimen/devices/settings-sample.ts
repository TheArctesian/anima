import type { DeviceDetail } from '$lib/devices/api';
import type { HubResult } from '$lib/hub/hub';
import type { UpdateDeviceInput } from '$lib/hub/generated/graphql';

/** A synthetic Linux workstation with SSH and VNC on, as the hub would send it, for the Settings lane (D17). */
export const SAMPLE_DEVICE: DeviceDetail = {
  id: 'specimen-pop-os',
  name: 'pop-os',
  host: '100.64.0.3',
  ip: null,
  kind: 'workstation',
  os: 'LINUX',
  distribution: 'pop',
  tags: ['lab', 'gpu'],
  maintenance: false,
  status: 'UP',
  site: { id: 'specimen-home', name: 'Home' },
  ssh: {
    enabled: true,
    port: 22,
    username: 'okita',
    auth: 'PASSWORD',
    credentialStored: true,
    hostKey: { algorithm: 'ecdsa-sha2-nistp256', fingerprintSha256: 'SHA256:dJSdNiHJtVlBobL+mSjapOl7CgO/JNfSu4qORr1jXqA', firstSeenAt: '2026-11-04T09:00:00Z' },
  },
  macAddress: null,
  webInterfaces: [],
  webInterfaceSuggestions: [],
  metricsSource: 'SSH',
  accessWarnings: ['UNENCRYPTED_VNC'],
  rdp: {
    enabled: false,
    port: 3389,
    username: '',
    domain: null,
    security: 'ANY',
    ignoreCertificate: false,
    resizeMethod: 'DISPLAY_UPDATE',
    audioOut: true,
    microphone: false,
    sharedDrive: false,
    credentialStored: false,
    serverCertificate: null,
  },
  vnc: { enabled: true, port: 5900, username: 'okita', credentialStored: true },
  transfer: { upload: true, download: true, copy: true, paste: false },
  day: 0.9995,
  week: 0.99,
  month: null,
};

/** The name the lane's fake hub refuses, so the refused state can be seen. */
export const REFUSED_NAME = 'atlas';

/** What the hub would store for `input`: each part it names replaces the device's, the rest stays. */
export function updated(device: DeviceDetail, input: UpdateDeviceInput): HubResult<DeviceDetail> {
  if (input.name === REFUSED_NAME) {
    return { ok: false, error: { code: 'VALIDATION', message: 'Invalid input', fields: [{ field: 'name', message: 'Another device has this name.' }] } };
  }
  const { ssh, rdp, vnc, transfer, ...plain } = input;
  // Only the synthetic lane builds a device this way, so a part the input leaves empty keeps the device's value.
  const value = {
    ...device,
    ...present(plain),
    ssh: { ...device.ssh, ...present(ssh) },
    rdp: { ...device.rdp, ...present(rdp) },
    vnc: { ...device.vnc, ...present(vnc) },
    transfer: { ...device.transfer, ...present(transfer) },
  } as DeviceDetail;
  return { ok: true, value };
}

/** The parts of an input that it sets. */
function present(part: object | null | undefined): Record<string, unknown> {
  return Object.fromEntries(Object.entries(part ?? {}).filter(([, value]) => value !== undefined && value !== null));
}
