// The switches of the open ports (HMD-049): which ports a computer offers, and the system setting each one needs. Pure.

/** The systems that name their settings differently. */
export type System = 'mac' | 'linux' | 'windows';

/** One port that the person can turn on for the hub. */
export interface PortRow {
  readonly port: number;
  readonly label: string;
  /** The setting of the system that must be on as well: Anima installs no program for a session (HMD-049). */
  readonly needs: string;
  readonly on: boolean;
}

const NEEDS: Readonly<Record<System, Readonly<Record<number, string>>>> = {
  mac: {
    22: 'Needs Remote Login: System Settings, General, Sharing.',
    5900: 'Needs Screen Sharing: System Settings, General, Sharing.',
  },
  linux: {
    22: 'Needs an SSH server on this computer, such as OpenSSH.',
    5900: 'Needs a VNC server on this computer, such as the screen sharing of your desktop.',
  },
  windows: {
    22: 'Needs OpenSSH Server: Settings, System, Optional features.',
    5900: 'Needs a VNC server on this computer.',
    3389: 'Needs Remote Desktop: Settings, System, Remote Desktop.',
  },
};

const LABELS: Readonly<Record<number, string>> = {
  22: 'SSH (port 22)',
  5900: 'Screen sharing (port 5900)',
  3389: 'Remote Desktop (port 3389)',
};

/** The system of this computer from the user agent of its web view. */
export function systemOf(userAgent: string): System {
  if (/Windows/.test(userAgent)) return 'windows';
  if (/Macintosh|Mac OS X/.test(userAgent)) return 'mac';
  return 'linux';
}

/** The rows for `system`, in port order, each on when `open` holds its port. Remote Desktop only on Windows. */
export function portRows(system: System, open: readonly number[]): readonly PortRow[] {
  return Object.entries(NEEDS[system]).map(([port, needs]) => {
    const number = Number(port);
    return { port: number, label: LABELS[number] ?? `Port ${port}`, needs, on: open.includes(number) };
  });
}
