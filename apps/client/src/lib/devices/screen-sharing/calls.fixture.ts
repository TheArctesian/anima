import { vi } from 'vitest';
import type { ThisMachine } from '$lib/desktop/this-machine';
import type { DeviceDetail } from '$lib/devices/api';
import type { ConnectionTest } from '$lib/sessions/api';
import type { HubResult } from '$lib/hub/hub';
import type { ScreenSharingCalls, SetupDevice } from './setup';

export const MAC: ThisMachine = { name: 'studio', user: 'ada', os: 'MACOS', vpnAddress: '100.64.0.5', sshListening: false };

/** A device whose host is this computer's VPN address (R-060). */
export const STUDIO: SetupDevice = { id: 'd1', name: 'Studio', host: '100.64.0.5', os: 'MACOS', vnc: { enabled: false } };

/** Another computer. */
export const NAS: SetupDevice = { id: 'd2', name: 'Nas', host: '100.64.0.9', os: 'LINUX', vnc: { enabled: false } };

export const UPDATED = { id: 'd1', name: 'Studio' } as DeviceDetail;

export function answer(reachable: boolean): HubResult<ConnectionTest> {
  return { ok: true, value: { reachable, authenticated: false, hostKeyChanged: false } } as HubResult<ConnectionTest>;
}

/** A clock that runs a timer only when the test says so. */
export function fakeClock() {
  const timers: { ms: number; run: () => void; stopped: boolean }[] = [];
  return {
    after: (ms: number, run: () => void) => {
      const timer = { ms, run, stopped: false };
      timers.push(timer);
      return () => {
        timer.stopped = true;
      };
    },
    /** Runs every timer that is waiting, as the interval passing would. */
    tick: () => {
      for (const timer of timers.splice(0)) if (!timer.stopped) timer.run();
    },
    waiting: () => timers.filter((timer) => !timer.stopped).length,
  };
}

/** Lets promises that already resolved run their continuations. */
export async function settle(): Promise<void> {
  for (let turn = 0; turn < 10; turn += 1) await Promise.resolve();
}

/** Calls whose answers the test sets with `mockResolvedValue` and the like; the defaults are this Mac, on the desktop, not answering. */
export function fakeCalls() {
  const clock = fakeClock();
  const calls = {
    test: vi.fn<ScreenSharingCalls['test']>(() => Promise.resolve(answer(false))),
    update: vi.fn<ScreenSharingCalls['update']>(() => Promise.resolve({ ok: true, value: UPDATED })),
    thisMachine: vi.fn<ScreenSharingCalls['thisMachine']>(() => Promise.resolve(MAC)),
    sharingAvailable: vi.fn<ScreenSharingCalls['sharingAvailable']>(() => true),
    openSharing: vi.fn<ScreenSharingCalls['openSharing']>(() => Promise.resolve()),
    after: clock.after,
    intervalMs: 3000,
  };
  return { calls, clock };
}
