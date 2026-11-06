import type { MetricsSource, PendingDeviceFields, RdpResizeMethod, RdpSecurity } from '../../hub-pending';

/** The K1 device fields as the form reads them; each may be missing until the devices client queries it (D-3). */
export type RemoteFields = Partial<PendingDeviceFields>;

export const DEFAULT_RDP_PORT = '3389';
export const DEFAULT_VNC_PORT = '5900';

/** What the operator typed for remote desktop, Wake-on-LAN, metrics and transfer (Section 6.2). Ports stay text. */
export interface RemoteValues {
  readonly rdp: {
    readonly enabled: boolean;
    readonly port: string;
    readonly username: string;
    readonly domain: string;
    readonly security: RdpSecurity;
    readonly ignoreCertificate: boolean;
    readonly resizeMethod: RdpResizeMethod;
    readonly audioOut: boolean;
    readonly microphone: boolean;
    readonly sharedDrive: boolean;
  };
  readonly vnc: { readonly enabled: boolean; readonly port: string; readonly username: string };
  /** As typed; `normalizedMac` gives the stored form (RA-031). */
  readonly macAddress: string;
  readonly metricsSource: MetricsSource;
  readonly transfer: { readonly upload: boolean; readonly download: boolean; readonly copy: boolean; readonly paste: boolean };
}

/** Typed passwords for RDP and VNC. Never filled from the hub (DEV-011); empty means "keep the stored one". */
export interface RemoteSecrets {
  readonly rdpPassword: string;
  readonly vncPassword: string;
}

export function emptyRemoteSecrets(): RemoteSecrets {
  return { rdpPassword: '', vncPassword: '' };
}

/** The values of a device that has no K1 fields: the K1 defaults. */
export function defaultRemoteValues(): RemoteValues {
  return {
    rdp: {
      enabled: false,
      port: DEFAULT_RDP_PORT,
      username: '',
      domain: '',
      security: 'NLA',
      ignoreCertificate: false,
      resizeMethod: 'DISPLAY_UPDATE',
      audioOut: true,
      microphone: false,
      sharedDrive: false,
    },
    vnc: { enabled: false, port: DEFAULT_VNC_PORT, username: '' },
    macAddress: '',
    metricsSource: 'NONE',
    transfer: { upload: true, download: true, copy: true, paste: true },
  };
}

/** The remote values of a saved device; a K1 field the device lacks reads as its default (D-3). */
export function remoteValues(fields: RemoteFields): RemoteValues {
  const base = defaultRemoteValues();
  const { rdp, vnc } = fields;
  return {
    rdp:
      rdp === undefined
        ? base.rdp
        : {
            enabled: rdp.enabled,
            port: String(rdp.port),
            username: rdp.username,
            domain: rdp.domain ?? '',
            security: rdp.security,
            ignoreCertificate: rdp.ignoreCertificate,
            resizeMethod: rdp.resizeMethod,
            audioOut: rdp.audioOut,
            microphone: rdp.microphone,
            sharedDrive: rdp.sharedDrive,
          },
    vnc: vnc === undefined ? base.vnc : { enabled: vnc.enabled, port: String(vnc.port), username: vnc.username ?? '' },
    macAddress: fields.macAddress ?? '',
    metricsSource: fields.metricsSource ?? base.metricsSource,
    transfer:
      fields.transfer === undefined
        ? base.transfer
        : { upload: fields.transfer.upload, download: fields.transfer.download, copy: fields.transfer.copy, paste: fields.transfer.paste },
  };
}

const MAC_PATTERN = /^[0-9a-f]{2}([:-])(?:[0-9a-f]{2}\1){4}[0-9a-f]{2}$/i;

/** RA-031: whether `text` is empty or six hex pairs joined by ':' or '-' (one kind of separator). */
export function isValidMac(text: string): boolean {
  const typed = text.trim();
  return typed === '' || MAC_PATTERN.test(typed);
}

/** RA-031: lower-case aa:bb:cc:dd:ee:ff; text that is not an address stays as typed (trimmed) so validation can report it. */
export function normalizedMac(text: string): string {
  const typed = text.trim();
  return MAC_PATTERN.test(typed) ? typed.toLowerCase().replaceAll('-', ':') : typed;
}

/** Remote values as they will be sent: trimmed text, normalized address. Secrets are not part of it. */
export function normalizedRemote(values: RemoteValues): RemoteValues {
  return {
    ...values,
    rdp: { ...values.rdp, port: values.rdp.port.trim(), username: values.rdp.username.trim(), domain: values.rdp.domain.trim() },
    vnc: { ...values.vnc, port: values.vnc.port.trim(), username: values.vnc.username.trim() },
    macAddress: normalizedMac(values.macAddress),
  };
}
