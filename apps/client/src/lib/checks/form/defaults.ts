import { checkDefaultsApi, type CheckDefaults } from '$lib/checks/api';
import { useHub } from '$lib/hub/context';
import type { HubResult } from '$lib/hub/hub';

/**
 * The call that reads the hub's default check settings (D-26), or `null` when the form has no hub above it.
 * Call it while the component is created; a form without a hub starts from the K1 input defaults.
 */
export function defaultsReader(): (() => Promise<HubResult<CheckDefaults>>) | null {
  try {
    return checkDefaultsApi(useHub()).get;
  } catch {
    // No hub in the context is a state, not a failure: nothing was asked, so nothing is lost.
    return null;
  }
}
