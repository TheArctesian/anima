import { DevicePolicy, SetDevicePolicy } from '$lib/devices/api';
import { useHub } from '$lib/hub/context';
import type { Hub, HubResult } from '$lib/hub/hub';
import type { PolicyInput, StoredPolicy } from './policy';

/** The two calls of the session policy: the hub's pending fields have their own documents (F-3). */
export interface PolicyCalls {
  load: (deviceId: string) => Promise<HubResult<StoredPolicy>>;
  save: (deviceId: string, input: PolicyInput) => Promise<HubResult<StoredPolicy>>;
}

const NOT_FOUND: HubResult<never> = { ok: false, error: { code: 'NOT_FOUND', message: 'No such device' } };

export function policyCalls(hub: Hub): PolicyCalls {
  return {
    load: async (deviceId) => {
      const result = await hub.request(DevicePolicy, { id: deviceId });
      if (!result.ok) return result;
      return result.value.device === null ? NOT_FOUND : { ok: true, value: result.value.device };
    },
    save: async (deviceId, input) => {
      const result = await hub.request(SetDevicePolicy, { id: deviceId, input });
      return result.ok ? { ok: true, value: result.value.updateDevice } : result;
    },
  };
}

/** Svelte's error for a context nobody set; its message carries the name in development and the docs URL in production. */
function isMissingContext(error: unknown): boolean {
  return error instanceof Error && error.message.includes('missing_context');
}

/** The calls of the hub above the form, or `null` when there is none: the form then has no policy to edit. Call it while the component is created. */
export function contextPolicyCalls(): PolicyCalls | null {
  try {
    return policyCalls(useHub());
  } catch (error) {
    if (isMissingContext(error)) return null;
    throw error;
  }
}
