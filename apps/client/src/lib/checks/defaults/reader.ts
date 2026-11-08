import { checkDefaultsApi, type CheckDefaults } from '$lib/checks/api';
import { useHub } from '$lib/hub/context';

/** Svelte's error for a context nobody set; its message carries the name in development and the docs URL in production. */
function isMissingContext(error: unknown): boolean {
  return error instanceof Error && error.message.includes('missing_context');
}

/**
 * The call that reads the hub's default check settings (D-26), or `null` when the form has no hub above it.
 * Call it while the component is created. The call gives `null` when the hub has no answer, whether it refuses,
 * is unreachable or throws: a new check then keeps the K1 input defaults.
 */
export function defaultsReader(): (() => Promise<CheckDefaults | null>) | null {
  let api;
  try {
    api = checkDefaultsApi(useHub());
  } catch (error) {
    if (isMissingContext(error)) return null; // No hub above the form is a state, not a failure: nothing is asked.
    throw error;
  }
  return async () => {
    try {
      const result = await api.get();
      return result.ok ? result.value : null;
    } catch {
      return null; // The fallback is the K1 input defaults; the form shows no error for a read it never needed.
    }
  };
}
