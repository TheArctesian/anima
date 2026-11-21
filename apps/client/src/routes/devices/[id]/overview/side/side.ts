import type { DeviceDetail } from '$lib/devices/api';
import { osLabel } from '$lib/devices/os/os';
import type { SessionProtocol } from '$lib/sessions/tabs';
import { connectActions, headerLine, type HeaderDevice, type ProtocolAction } from '../../header/header';

/** What the cards beside the Overview read from a device (R-224). */
export type SideDevice = HeaderDevice &
  Pick<DeviceDetail, 'kind' | 'tags' | 'metricsSource' | 'metricsStale' | 'daemonVersion' | 'sessionLimit'> & {
    readonly ssh: Pick<DeviceDetail['ssh'], 'enabled' | 'hostKey' | 'port' | 'username'>;
    readonly rdp: Pick<DeviceDetail['rdp'], 'enabled' | 'port'>;
    readonly vnc: Pick<DeviceDetail['vnc'], 'enabled' | 'username' | 'port'>;
  };

/** One way in to the device: its name, one line of detail, and what its button does. */
export interface WayIn {
  readonly key: string;
  readonly name: string;
  readonly detail: string;
  /** The words of the button; `null` draws no button, because nothing here can start that way. */
  readonly button: string | null;
  /** The first way that opens a session is the main one. */
  readonly main: boolean;
  readonly action: ProtocolAction | null;
  readonly webInterface: DeviceDetail['webInterfaces'][number] | null;
}

const BUTTON: Readonly<Record<ProtocolAction['mode'], string>> = { connect: 'Open', test: 'Test', 'set-up': 'Set up' };

function protocolDetail(device: SideDevice, action: ProtocolAction): string {
  if (action.mode === 'set-up') return 'Not set up';
  const port = `port ${String(device[action.protocol === 'SSH' ? 'ssh' : action.protocol === 'RDP' ? 'rdp' : 'vnc'].port)}`;
  if (action.protocol === 'SSH') {
    const login = device.ssh.username == null || device.ssh.username === '' ? port : `${device.ssh.username} · ${port}`;
    return action.mode === 'test' ? `${login} · host key not accepted yet` : login;
  }
  if (action.protocol === 'VNC' && device.accessWarnings.includes('UNENCRYPTED_VNC')) return `${port} · not encrypted`;
  return port;
}

/** "https, port 3000" from the address of a web interface; the address itself when it cannot be read. */
export function webDetail(url: string): string {
  if (!URL.canParse(url)) return url;
  const { protocol, port } = new URL(url);
  const scheme = protocol.replace(':', '');
  return `${scheme}, port ${port === '' ? (scheme === 'https' ? '443' : '80') : port}`;
}

/** The one row of the hub's own device in place of SSH, RDP and VNC: nothing to set up (HMD-045, HMD-046). */
const NO_WAY_IN: WayIn = {
  key: 'none',
  name: 'SSH, RDP and VNC',
  detail: 'No session reaches the container of the hub',
  button: null,
  main: false,
  action: null,
  webInterface: null,
};

/**
 * Each way in, in the order SSH, RDP, VNC, then each web interface (WEB-001): what the bar's connect button and its
 * menu hold, one row each. `web` is false where the page has no way to open a web interface. The hub's own device has
 * one row that says it has none (HMD-046).
 */
export function waysIn(device: SideDevice, web: boolean): readonly WayIn[] {
  const actions = connectActions(headerLine(device), device.name);
  if (actions === null) return [NO_WAY_IN, ...webWays(device, web)];
  const { main, others } = actions;
  const order: readonly SessionProtocol[] = ['SSH', 'RDP', 'VNC'];
  const protocols = [main, ...others].sort((a, b) => order.indexOf(a.protocol) - order.indexOf(b.protocol));
  return [
    ...protocols.map((action) => ({
      key: action.protocol,
      name: action.protocol,
      detail: protocolDetail(device, action),
      button: BUTTON[action.mode],
      main: action === main && action.mode === 'connect',
      action,
      webInterface: null,
    })),
    ...webWays(device, web),
  ];
}

function webWays(device: SideDevice, web: boolean): WayIn[] {
  return device.webInterfaces.map((webInterface) => ({
    key: `web:${webInterface.id}`,
    name: webInterface.label,
    detail: webDetail(webInterface.url),
    button: web ? 'Open' : null,
    main: false,
    action: null,
    webInterface,
  }));
}

export interface Fact {
  readonly label: string;
  readonly text: string;
}

/** The daemon in words: its version and whether it sends, or what reads the metrics in its place. */
export function daemonText(device: Pick<SideDevice, 'metricsSource' | 'metricsStale' | 'daemonVersion'>): string {
  if (device.metricsSource === 'SSH') return 'Not installed. Metrics are read over SSH';
  if (device.metricsSource !== 'DAEMON') return 'Not installed';
  const version = device.daemonVersion ?? 'Version not known';
  return device.metricsStale ? `${version}, not sending now` : `${version}, sending metrics`;
}

/** What the device is: the facts of the card "About"; the tags are drawn apart, as marks. */
export function aboutFacts(device: SideDevice): readonly Fact[] {
  return [
    { label: 'System', text: osLabel(device.os, device.distribution) },
    { label: 'Kind', text: device.kind },
    { label: 'Daemon', text: daemonText(device) },
    { label: 'Sessions', text: device.sessionLimit == null ? 'No limit' : `At most ${String(device.sessionLimit)} at once` },
  ];
}
