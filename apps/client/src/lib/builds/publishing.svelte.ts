import { createContext } from 'svelte';
import type { Hub } from '$lib/hub/hub';
import { releasePublishing } from './api';

/**
 * Whether the hub can publish a release of Anima.ing: it holds the release bucket and the updater key (PRO-037). A hub
 * that cannot refuses a release, so the client shows no Builds item and no builder row for it (PRO-038). Off until the
 * hub says otherwise, and off when it cannot be asked.
 */
export class ReleasePublishing {
  on = $state(false);

  constructor(private readonly hub: Pick<Hub, 'request'>) {}

  async load(): Promise<void> {
    this.on = await releasePublishing(this.hub);
  }
}

const [getPublishing, setPublishing] = createContext<ReleasePublishing>();

/** Make `publishing` available below; call once in the root layout of the desktop app. */
export function providePublishing(publishing: ReleasePublishing): void {
  setPublishing(publishing);
}

export function usePublishing(): ReleasePublishing {
  return getPublishing();
}
