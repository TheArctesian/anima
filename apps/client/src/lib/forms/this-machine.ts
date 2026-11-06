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
 * This computer if it can be offered in Add device: known to the shell and not yet a device. A device is this
 * computer when its host is the computer's VPN address (R-060; the machine ID of R-058 needs a K-change).
 * `null` too when the hosts cannot be read, since the app cannot then tell whether it would add a duplicate.
 */
export async function machineToOffer(
  read: () => Promise<ThisMachine | null>,
  hosts: () => Promise<HubResult<readonly string[]>>,
): Promise<ThisMachine | null> {
  const machine = await read();
  if (machine === null) return null;
  const known = await hosts();
  if (!known.ok) return null;
  return known.value.some((host) => normalizeHost(host) === machine.vpnAddress) ? null : machine;
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
