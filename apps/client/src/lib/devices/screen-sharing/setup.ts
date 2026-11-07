import type { DeviceDetail } from '$lib/devices/api';
import type { ThisMachine } from '$lib/desktop/thisMachine';
import { normalizeHost } from '$lib/forms/device/host';
import type { ConnectionTestInput, OperatingSystem, UpdateDeviceInput } from '$lib/hub/generated/graphql';
import type { HubResult } from '$lib/hub/hub';
import type { ConnectionTest } from '$lib/sessions/api';

/** The port the flow tests and turns on: every Screen Sharing and VNC server listens on 5900 (R-079). */
export const SCREEN_SHARING_PORT = 5900;

/** How often the flow asks again while the operator turns Screen Sharing on, in milliseconds (R-079). */
export const RETEST_INTERVAL_MS = 3000;

export const NO_ANSWER_TEXT = `The device did not answer on port ${String(SCREEN_SHARING_PORT)}. Either Screen Sharing (or the VNC server) is off on it, or the VPN rule does not allow port ${String(SCREEN_SHARING_PORT)}.`;

export const SHARING_PROMPT = 'Turn on Screen Sharing';

/** What the flow reads from a device. */
export interface SetupDevice {
  readonly id: string;
  readonly name: string;
  readonly host: string;
  readonly os: OperatingSystem;
  readonly vnc: { readonly enabled: boolean };
}

/** R-079: only a macOS or Linux device whose VNC is off is offered the action. */
export function offersScreenSharing(device: Pick<SetupDevice, 'os' | 'vnc'>): boolean {
  return (device.os === 'MACOS' || device.os === 'LINUX') && !device.vnc.enabled;
}

/** R-060: the device whose host is this computer's VPN address is this computer. Outside the desktop app `machine` is null. */
export function isThisComputer(device: Pick<SetupDevice, 'host'>, machine: ThisMachine | null): machine is ThisMachine {
  return machine !== null && normalizeHost(device.host) === machine.vpnAddress;
}

/** The unsaved test of the VNC port (WP-35 D-1). It carries no credential, and a stale stored port cannot change it. */
export function screenSharingTest(device: Pick<SetupDevice, 'host'>): ConnectionTestInput {
  return { host: device.host, protocol: 'VNC', vnc: { enabled: true, port: SCREEN_SHARING_PORT } };
}

/** The update that turns VNC on with the defaults; no password. The username is this computer's user, or none. */
export function enableVnc(user: string | null): UpdateDeviceInput {
  return { vnc: { enabled: true, port: SCREEN_SHARING_PORT, username: user === null || user === '' ? null : user } };
}

/** What a card or row needs to offer the flow: the calls, and what to do once the device has VNC on (R-079). */
export interface CardScreenSharing {
  readonly calls: ScreenSharingCalls;
  readonly onReady: (device: DeviceDetail, button: HTMLElement) => void;
}

/** Where the flow stands; the button and its status line show it. */
export type SetupState =
  | { readonly kind: 'idle' }
  | { readonly kind: 'testing' }
  /** `problem` is the fixed text of a failure to open System Settings, or null. */
  | { readonly kind: 'waiting'; readonly problem: string | null }
  | { readonly kind: 'starting' }
  | { readonly kind: 'failed'; readonly message: string };

/** The sentence a screen reader hears for each stage; failures have their own alert, so they add nothing here. */
export function liveText(state: SetupState): string {
  switch (state.kind) {
    case 'testing':
      return 'Checking port 5900.';
    case 'waiting':
      return `${SHARING_PROMPT} in System Settings. Checking again every few seconds.`;
    case 'starting':
      return 'Turning on screen sharing.';
    default:
      return '';
  }
}

/** Whether the flow is working, so the button must not start it again. */
export function isBusy(state: SetupState): boolean {
  return state.kind !== 'idle' && state.kind !== 'failed';
}

/** Runs `run` once after `ms`; the result stops it. The flow takes it as a parameter, so tests keep their own clock. */
export type After = (ms: number, run: () => void) => () => void;

/** What the flow calls outside itself: the hub, this computer, System Settings and the clock. */
export interface ScreenSharingCalls {
  test: (input: ConnectionTestInput) => Promise<HubResult<ConnectionTest>>;
  update: (id: string, input: UpdateDeviceInput) => Promise<HubResult<DeviceDetail>>;
  thisMachine: () => Promise<ThisMachine | null>;
  /** Whether System Settings can open at Sharing: the desktop app on a Mac. */
  sharingAvailable: () => boolean;
  /** Opens System Settings at Sharing; rejects with a `SystemSettingsError`. */
  openSharing: () => Promise<void>;
  after: After;
  intervalMs: number;
}
