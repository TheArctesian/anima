import type { Hub, HubError } from '$lib/hub/hub';
import { devicesApi } from '../client';
import { siteOptions, type MenuOption } from './toolbar';

/** The hub's sites for the Site menu, and why there are none when they cannot be had (VPN-002). */
export class Sites {
  #all = $state.raw<readonly { id: string; name: string }[]>([]);
  error = $state.raw<HubError | null>(null);
  readonly #api;

  constructor(hub: Hub) {
    this.#api = devicesApi(hub);
  }

  /** "All sites" and each site; empty while loading, after a failure and when there is no site, so the note shows. */
  get options(): readonly MenuOption[] {
    return this.#all.length === 0 ? [] : siteOptions(this.#all);
  }

  /** The text for an empty menu. */
  get note(): string {
    return this.error?.message ?? 'No sites yet';
  }

  async load(): Promise<void> {
    const result = await this.#api.sites();
    if (result.ok) this.#all = result.value;
    else this.error = result.error;
  }
}
