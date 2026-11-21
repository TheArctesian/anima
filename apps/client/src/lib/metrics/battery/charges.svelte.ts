import { SvelteMap } from 'svelte/reactivity';
import type { Hub, HubError } from '$lib/hub/hub';
import { batteryApi } from './api';
import { mergeCharges, NO_CHARGES, type Charges } from './battery';

/**
 * The latest charge of each battery of every device, from the batches as the hub pushes them (MET-032). A device has
 * no charge until its first batch after `start`: its daemon pushes one every 10 seconds (MET-018).
 */
export class LiveCharges {
  readonly #byDevice = new SvelteMap<string, Charges>();
  error = $state.raw<HubError | null>(null);
  #stop: (() => void) | undefined;

  /** The charges of device `id`; `undefined` before a batch of it came. */
  of(id: string): Charges | undefined {
    return this.#byDevice.get(id);
  }

  /** Starts listening; a second call while it listens changes nothing. */
  start(hub: Pick<Hub, 'subscribe'>): void {
    if (this.#stop !== undefined) return;
    this.#stop = batteryApi(hub).watch({
      next: ({ deviceId, samples }) => {
        const known = this.#byDevice.get(deviceId) ?? NO_CHARGES;
        const next = mergeCharges(known, samples);
        if (next !== known) this.#byDevice.set(deviceId, next);
      },
      error: (failure) => {
        this.error = failure;
      },
      complete: () => undefined,
    });
  }

  stop(): void {
    this.#stop?.();
    this.#stop = undefined;
  }
}
