import { readCloudDevice, type CloudDevice } from '$lib/desktop/cloud/device';
import { devicesApi } from '$lib/devices/client';
import { comparable } from '$lib/hosted/this-computer';
import type { MetricsSource } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';
import { linkApi } from '$lib/link/api';
import { metricsApi } from '$lib/metrics/api';
import { daemonBundled, installDaemon, type DaemonShellProblem, type DaemonShellResult } from './shell';

/** The most devices that are read to find this computer: one page of the hub. */
const ONE_PAGE = 200;

/** A device of the hub, as far as the install needs it. */
export interface HubDevice {
  readonly id: string;
  readonly host: string;
  readonly metricsSource: MetricsSource;
}

/** What the install on this computer needs; each part is injected, so that tests need no shell and no hub. */
export interface DaemonHereParts {
  /** Whether this build of the app carries the daemon. */
  readonly bundled: () => Promise<boolean>;
  /** This computer as the shell reads it; `null` when it cannot. */
  readonly device: () => Promise<CloudDevice | null>;
  /** The devices on the link of the hub, with the name the hub gave each one. */
  readonly linkDevices: () => Promise<HubResult<readonly { readonly name: string; readonly address: string }[]>>;
  /** The devices of the hub. A hub with more than one page of devices gives the first page. */
  readonly devices: () => Promise<HubResult<readonly HubDevice[]>>;
  /** A new enrolment code for the device; it works once. */
  readonly enrol: (deviceId: string) => Promise<HubResult<{ readonly code: string }>>;
  /** Has the shell install the daemon with the code, enrolled at the hub that the app reaches at `origin`. */
  readonly install: (code: string, origin: string) => Promise<DaemonShellResult>;
  /** The address at which the app reaches the hub. */
  readonly origin: string;
  /** Waits; a test passes one that does not. Default: a timer. */
  readonly wait?: (milliseconds: number) => Promise<void>;
}

/**
 * The install can start before it can work: the person is not signed in yet, this computer is not a device of the hub
 * yet (HMD-044), or the link has no socket for the daemon yet, which the shell answers with `origin` (the unit of the
 * link is made a short time after the sign-in, HMD-058). The install then waits and tries again, this many times in all.
 */
const TRIES = 5;
const WAIT_MS = 30_000;
const timer = (milliseconds: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, milliseconds));

/**
 * What the install did: it installed the daemon; the daemon of this computer reports already; there was nothing to
 * do it with (no daemon in this build, or this computer is not a device of the hub yet); or the step that failed.
 */
export type DaemonHere =
  | { readonly kind: 'installed' }
  | { readonly kind: 'present' }
  | { readonly kind: 'not-ready' }
  | { readonly kind: 'failed'; readonly step: 'enrol' | DaemonShellProblem };

/** The device of the hub whose host is the link address of this computer, or `null`. */
async function deviceHere(parts: DaemonHereParts): Promise<HubDevice | null> {
  const computer = await parts.device();
  if (computer === null) return null;
  const linked = await parts.linkDevices();
  if (!linked.ok) return null;
  const address = linked.value.find((link) => link.name === computer.name)?.address;
  if (address === undefined || address === '') return null;
  const devices = await parts.devices();
  if (!devices.ok) return null;
  const mine = comparable(address);
  return devices.value.find((device) => comparable(device.host) === mine) ?? null;
}

/** One turn: what it did, and whether a later turn of this start can do better. */
interface Turn {
  readonly did: DaemonHere;
  readonly again: boolean;
}

async function turn(parts: DaemonHereParts): Promise<Turn> {
  // Not signed in yet, or this computer is not a device of the hub yet: a later turn finds it. No code is made.
  const device = await deviceHere(parts);
  if (device === null) return { did: { kind: 'not-ready' }, again: true };
  if (device.metricsSource === 'DAEMON') return { did: { kind: 'present' }, again: false };
  // The code lives in this turn only: it goes to the shell and nowhere else (LOG-007). The shell checks the address
  // before it uses the code, so a code of a turn that waits for the socket is never used and expires at the hub.
  const made = await parts.enrol(device.id);
  if (!made.ok) return { did: { kind: 'failed', step: 'enrol' }, again: false };
  const done = await parts.install(made.value.code, parts.origin);
  if (done.ok) return { did: { kind: 'installed' }, again: false };
  return { did: { kind: 'failed', step: done.problem }, again: done.problem === 'origin' };
}

async function install(parts: DaemonHereParts): Promise<DaemonHere> {
  if (!(await parts.bundled())) return { kind: 'not-ready' };
  for (let attempt = 1; ; attempt += 1) {
    const { did, again } = await turn(parts);
    if (!again || attempt >= TRIES) return did;
    await (parts.wait ?? timer)(WAIT_MS);
  }
}

/** The install that runs now; a second page that opens meanwhile waits for it, so one code is made, not two. */
let running: Promise<DaemonHere> | null = null;
/** Set once an install of this start of the app has ended: the app tries once for each start (HMD-053). */
let tried = false;

/**
 * Installs the daemon on the computer the app runs on, enrolled at the hosted hub, with no question (HMD-053, R-240):
 * the daemon that the app carries, as a unit of the user. It does nothing when the device of this computer has the
 * daemon as its metrics source. It runs once for each start of the app and waits for what it needs (see `TRIES`); a
 * failure is quiet, and the next start tries again. The enrolment code is never shown, stored or logged.
 */
export function installDaemonHere(parts: DaemonHereParts): Promise<DaemonHere> {
  if (tried && running === null) return Promise.resolve({ kind: 'not-ready' });
  running ??= install(parts)
    .catch((): DaemonHere => ({ kind: 'failed', step: 'failed' }))
    .finally(() => {
      tried = true;
      running = null;
    });
  return running;
}

/** The install for the computer the app runs on, with the calls of `hub` and of the desktop shell. */
export function installDaemonOnThisComputer(hub: Hub): Promise<DaemonHere> {
  return installDaemonHere({
    bundled: daemonBundled,
    device: readCloudDevice,
    linkDevices: () => linkApi(hub).devices(),
    devices: async () => {
      const page = await devicesApi(hub).list({ first: ONE_PAGE });
      return page.ok ? { ok: true, value: page.value.edges.map((edge) => edge.node) } : page;
    },
    enrol: (deviceId) => metricsApi(hub).enrolDaemon(deviceId),
    install: installDaemon,
    origin: hub.address.origin,
  });
}

/** For tests: the next call runs as the first of a start of the app. */
export function forgetDaemonHere(): void {
  tried = false;
  running = null;
}
