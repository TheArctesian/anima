import { devicesApi } from '$lib/devices/client';
import type { Hub, HubResult } from '$lib/hub/hub';
import { sessionsApi } from '$lib/sessions/api';
import { webInterfacesApi } from '$lib/web-interfaces/api';
import type { Detail } from '../detail.svelte';
import { settingsCalls, type SettingsCalls } from '../settings/calls';

/** The hub calls the Settings tab makes beyond saving: the connection test and the confirmed delete (DEV-014). */
export interface DetailCalls {
  readonly settings: SettingsCalls;
  readonly onTest: ReturnType<typeof sessionsApi>['testConnection'];
  readonly onDelete: () => Promise<HubResult<unknown>>;
}

/** Builds the calls of `detail`'s Settings tab; `onDeleted` runs after the hub deleted the device. */
export function detailCalls(hub: Hub, detail: Detail, onDeleted: () => void): DetailCalls {
  const connections = sessionsApi(hub);
  const settingsHub = { devices: devicesApi(hub), sessions: connections, webInterfaces: webInterfacesApi(hub) };
  return {
    settings: settingsCalls(detail.id, settingsHub, detail),
    onTest: connections.testConnection,
    onDelete: () => detail.deleteDevice(onDeleted),
  };
}
