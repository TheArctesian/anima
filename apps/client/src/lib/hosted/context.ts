import { useHub } from '$lib/hub/context';
import type { HubAddress } from '$lib/hub/hub';
import { hubIsHosted } from './mark';

/** Svelte's error for a context nobody set; its message carries the name in development and the docs URL in production. */
function isMissingContext(error: unknown): boolean {
  return error instanceof Error && error.message.includes('missing_context');
}

/**
 * Whether the hub above the component is hosted by Anima.ing; `false` when no hub is above it or it has no address.
 * Call it while the component is created (R-211).
 */
export function contextHubIsHosted(): boolean {
  try {
    const hub: { readonly address?: Pick<HubAddress, 'origin'> } = useHub();
    return hub.address !== undefined && hubIsHosted(hub.address.origin);
  } catch (error) {
    if (isMissingContext(error)) return false;
    throw error;
  }
}
