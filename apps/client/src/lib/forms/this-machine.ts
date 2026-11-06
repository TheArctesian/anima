import type { ThisMachine } from '$lib/desktop/thisMachine';
import type { HubResult } from '$lib/hub/error';
import { normalizeHost } from './device/host';

/** What Add device adds: the computer the app runs on, pre-filled, or another device typed in (R-060). */
export type MachineChoice = 'this' | 'another';

/** The name of this computer in the choice and the "Don't ask again" button: "Mac" on macOS. */
function noun(os: ThisMachine['os']): string {
  return os === 'MACOS' ? 'Mac' : 'computer';
}

/** The first option of the Add device choice. */
export function thisMachineLabel(os: ThisMachine['os']): string {
  return `This ${noun(os)}`;
}

/** The title of the modal when the app asks, unprompted, to add this computer. */
export function promptTitle(os: ThisMachine['os']): string {
  return `Add this ${noun(os)}?`;
}

/** The button that stops the prompt for good on this computer. */
export function dontAskLabel(os: ThisMachine['os']): string {
  return `Don't ask again on this ${noun(os)}`;
}

/**
 * What Add device can do about this computer (R-060): `offer` it, pre-filled; nothing because it is already a
 * `device`; or nothing because it is `unknown` (not the desktop app, off the VPN, or the hosts could not be read,
 * so the app cannot tell whether it would add a duplicate).
 */
export type MachineOffer =
  | { readonly kind: 'offer'; readonly machine: ThisMachine }
  | { readonly kind: 'device' }
  | { readonly kind: 'unknown' };

/**
 * Whether this computer can be offered in Add device: known to the shell and not yet a device. A device is this
 * computer when its host is the computer's VPN address (R-060; the machine ID of R-058 needs a K-change).
 */
export async function machineToOffer(
  read: () => Promise<ThisMachine | null>,
  hosts: () => Promise<HubResult<readonly string[]>>,
): Promise<MachineOffer> {
  const machine = await read();
  if (machine === null) return { kind: 'unknown' };
  const known = await hosts();
  if (!known.ok) return { kind: 'unknown' };
  const added = known.value.some((host) => normalizeHost(host) === machine.vpnAddress);
  return added ? { kind: 'device' } : { kind: 'offer', machine };
}

/** The machine an offer holds, or `null`. */
export function offeredMachine(offer: MachineOffer): ThisMachine | null {
  return offer.kind === 'offer' ? offer.machine : null;
}

/** The `localStorage` key of "Don't ask again on this Mac". */
export const DONT_ASK_KEY = 'anima.thisMachine.dontAsk';

/** The browser's `localStorage`, or `null` where reading it throws (privacy modes, no DOM). */
export function browserStorage(): Storage | null {
  try {
    return globalThis.localStorage;
  } catch {
    return null;
  }
}

/**
 * Whether the app may ask to add this computer (R-060): at most once per app start, and never after the
 * operator chose "Don't ask again". `storage` is read on each call, so a refusing store simply means "ask".
 */
export class ThisMachinePrompt {
  #askedThisRun = false;

  constructor(private readonly storage: () => Storage | null) {}

  /** Not yet asked since the app started, and not turned off on this computer. */
  get due(): boolean {
    return !this.#askedThisRun && !this.#turnedOff();
  }

  /** The prompt was shown; closing it is "Not now", so the next app start asks again. */
  asked(): void {
    this.#askedThisRun = true;
  }

  /** This computer is already a device: there is nothing to ask, so the rest of this run does not look again. */
  isADevice(): void {
    this.#askedThisRun = true;
  }

  /** "Don't ask again on this Mac". A store that refuses the write still stops the prompt for this run. */
  turnOff(): void {
    this.#askedThisRun = true;
    try {
      this.storage()?.setItem(DONT_ASK_KEY, 'true');
    } catch {
      // Storage refused (privacy mode, quota): the prompt returns at the next start, as after "Not now".
    }
  }

  #turnedOff(): boolean {
    try {
      return this.storage()?.getItem(DONT_ASK_KEY) === 'true';
    } catch {
      return false;
    }
  }
}

/** The prompt for this app start; the module lives as long as the app does. */
export const thisMachinePrompt = new ThisMachinePrompt(browserStorage);
