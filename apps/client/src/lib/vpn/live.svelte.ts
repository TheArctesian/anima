import type { HubError } from '$lib/hub/hub';
import type { VpnPicture, vpnApi } from './api';

/** The VPN picture: loaded once, then replaced by each `vpnChanged` the hub sends (VPN-005, SET-003). */
export class VpnLive {
  /** `undefined` while loading; `null` when the hub has no VPN status to give. */
  picture = $state<VpnPicture | null | undefined>(undefined);
  error = $state<HubError | null>(null);
  #unwatch: (() => void) | null = null;
  #stopped = false;

  constructor(private readonly api: ReturnType<typeof vpnApi>) {}

  /** Reads the picture and, on success, starts listening for new ones (once). */
  async load(): Promise<void> {
    this.#stopped = false;
    const result = await this.api.picture();
    if (this.#stopped) return;
    if (!result.ok) {
      this.error = result.error;
      return;
    }
    this.error = null;
    this.picture = result.value;
    this.#unwatch ??= this.api.watch({
      next: (picture) => {
        this.picture = picture;
        this.error = null;
      },
      error: (error) => {
        this.error = error;
      },
      complete: () => undefined,
    });
  }

  /** Stops listening. */
  stop(): void {
    this.#stopped = true;
    this.#unwatch?.();
    this.#unwatch = null;
  }
}
