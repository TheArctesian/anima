// Device and web interface read shapes that wait for the devices client documents (H-1). Codegen emits a type
// only for a field a running document selects, and lib/devices/api.ts does not select these yet; the hub does
// not serve them until WP-21 and WP-30. Each name is the one codegen will give it, so the move changes imports only.
// The K1 enums and inputs, and the check config shapes, come from `$lib/hub/generated/graphql` (WP-25 T5).
import type { MetricsSource, RdpResizeMethod, RdpSecurity } from '$lib/hub/generated/graphql';

// ---- devices
/** RA-020. */
export type AccessWarning = 'IGNORED_CERTIFICATE' | 'RDP_SECURITY' | 'UNENCRYPTED_VNC';

export interface ServerCertificate {
  readonly fingerprintSha256: string;
  readonly subject?: string | null;
  readonly notAfter?: string | null;
  readonly firstSeenAt: string;
}

export interface RdpSettings {
  readonly enabled: boolean;
  readonly port: number;
  readonly username: string;
  readonly domain?: string | null;
  readonly security: RdpSecurity;
  readonly ignoreCertificate: boolean;
  readonly resizeMethod: RdpResizeMethod;
  readonly audioOut: boolean;
  readonly microphone: boolean;
  readonly sharedDrive: boolean;
  readonly credentialStored: boolean;
  readonly serverCertificate?: ServerCertificate | null;
}

export interface VncSettings {
  readonly enabled: boolean;
  readonly port: number;
  /** macOS Screen Sharing needs one. */
  readonly username?: string | null;
  readonly credentialStored: boolean;
}

/** RA-032, RA-033. All default true. */
export interface TransferPolicy {
  readonly upload: boolean;
  readonly download: boolean;
  readonly copy: boolean;
  readonly paste: boolean;
}

/** The K1 `Device` fields the device form reads. */
export interface PendingDeviceFields {
  readonly rdp: RdpSettings;
  readonly vnc: VncSettings;
  /** Lower-case aa:bb:cc:dd:ee:ff (RA-031). */
  readonly macAddress?: string | null;
  readonly metricsSource: MetricsSource;
  readonly transfer: TransferPolicy;
  readonly accessWarnings: readonly AccessWarning[];
}

/** The K1 additions to `ConnectionTestResult`. */
export interface PendingConnectionTestResult {
  readonly serverCertificate?: ServerCertificate | null;
  readonly certificateChanged: boolean;
}

// ---- web interfaces
export type OpenIn = 'ANIMA_TAB' | 'SYSTEM_BROWSER';

export interface WebInterface {
  readonly id: string;
  readonly deviceId: string;
  readonly label: string;
  readonly url: string;
  readonly openIn: OpenIn;
  /** Pinned on accept (WEB-008). */
  readonly pinnedFingerprintSha256?: string | null;
  readonly pinnedAt?: string | null;
}

/** WEB-002. */
export interface WebInterfaceSuggestion {
  readonly checkId: string;
  readonly url: string;
  readonly label: string;
}

export interface WebInterfaceInput {
  /** Defaults to the URL's host. */
  label?: string | null;
  url: string;
  openIn?: OpenIn | null;
}
