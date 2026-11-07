import type { DeviceDetail } from '$lib/devices/api';
import type { ServerCertificate } from '$lib/sessions/api';
import type { WebInterface } from '$lib/web-interfaces/api';

/** The device after the hub stored or removed its RDP password (DEV-011). */
export function withRdpStored(device: DeviceDetail, credentialStored: boolean): DeviceDetail {
  return { ...device, rdp: { ...device.rdp, credentialStored } };
}

/** The device after the hub stored or removed its VNC password (DEV-011). */
export function withVncStored(device: DeviceDetail, credentialStored: boolean): DeviceDetail {
  return { ...device, vnc: { ...device.vnc, credentialStored } };
}

/** The device after the hub accepted this RDP server certificate (SEC-007). */
export function withServerCertificate(device: DeviceDetail, serverCertificate: ServerCertificate): DeviceDetail {
  return { ...device, rdp: { ...device.rdp, serverCertificate } };
}

/** The device with `web` in place of the interface with its ID, or added at the end (WEB-001). */
export function withWebInterface(device: DeviceDetail, web: WebInterface): DeviceDetail {
  const known = device.webInterfaces.some((other) => other.id === web.id);
  const webInterfaces = known
    ? device.webInterfaces.map((other) => (other.id === web.id ? web : other))
    : [...device.webInterfaces, web];
  return { ...device, webInterfaces };
}

/** The device without the web interface with this ID (WEB-001). */
export function withoutWebInterface(device: DeviceDetail, id: string): DeviceDetail {
  return { ...device, webInterfaces: device.webInterfaces.filter((web) => web.id !== id) };
}

/** The device without the suggestion of this check, once it is accepted or dismissed (WEB-002). */
export function withoutSuggestion(device: DeviceDetail, checkId: string): DeviceDetail {
  return { ...device, webInterfaceSuggestions: device.webInterfaceSuggestions.filter((offer) => offer.checkId !== checkId) };
}
