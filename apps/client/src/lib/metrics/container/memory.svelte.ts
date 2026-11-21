import type { Hub, HubError } from '$lib/hub/hub';
import { containerApi } from './api';
import { mergeMemoryBytes } from './container';

/**
 * The bytes of memory in use of the hub's own device, from its batches as the hub pushes them (HMD-073). They are
 * unknown until its first batch after `watch`: its daemon pushes one every 10 seconds (MET-018).
 */
export class LiveMemory {
  bytes = $state.raw<number | null>(null);
  error = $state.raw<HubError | null>(null);
  #deviceId: string | null = null;
  #stop: (() => void) | undefined;

  /** The bytes in use of device `id`; `null` for another device or before a batch came. */
  of(id: string): number | null {
    return id === this.#deviceId ? this.bytes : null;
  }

  /** Listens to device `deviceId`; the same device again changes nothing, a different one replaces it. */
  watch(hub: Pick<Hub, 'subscribe'>, deviceId: string): void {
    if (deviceId === this.#deviceId) return;
    this.stop();
    this.#deviceId = deviceId;
    this.#stop = containerApi(hub).watch(deviceId, {
      next: ({ samples }) => {
        this.bytes = mergeMemoryBytes(this.bytes, samples);
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
    this.#deviceId = null;
    this.bytes = null;
  }
}
