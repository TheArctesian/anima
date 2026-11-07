import type { DeviceDetail } from '$lib/devices/api';
import type { devicesApi } from '$lib/devices/client';
import type { WebInterfaceCalls } from '$lib/forms/device/web/WebInterfaces.svelte';
import type { DeviceFormValues } from '$lib/forms/device/values';
import type { HubResult } from '$lib/hub/hub';
import type { SessionsApi } from '$lib/sessions/api';
import type { webInterfacesApi } from '$lib/web-interfaces/api';
import { saveDevice, type DeviceSaveCalls } from '../save';
import {
  withoutSuggestion,
  withoutWebInterface,
  withRdpStored,
  withServerCertificate,
  withVncStored,
  withWebInterface,
} from './changes';

/** The domain clients the Settings tab calls; the route passes `devicesApi`, `sessionsApi` and `webInterfacesApi` of its hub. */
export interface SettingsHub {
  readonly devices: Pick<
    ReturnType<typeof devicesApi>,
    'update' | 'setSshCredential' | 'clearSshCredential' | 'clearRdpCredential' | 'clearVncCredential'
  >;
  readonly sessions: Pick<SessionsApi, 'acceptHostKey' | 'acceptServerCertificate' | 'setCredential'>;
  readonly webInterfaces: ReturnType<typeof webInterfacesApi>;
}

/** The device the page shows; it is read again when each answer arrives, so a late answer changes the latest device. */
export interface ShownDevice {
  readonly device: DeviceDetail | null;
  setDevice: (next: DeviceDetail) => void;
}

/** Every hub call the Settings tab makes; each one that succeeds puts what the hub stored into the shown device. */
export interface SettingsCalls {
  onSave: (values: DeviceFormValues) => Promise<HubResult<unknown>>;
  onAcceptHostKey: (fingerprint: string) => Promise<HubResult<unknown>>;
  onAcceptServerCertificate: (fingerprint: string) => Promise<HubResult<unknown>>;
  onClearCredential: () => Promise<HubResult<unknown>>;
  onClearRdpCredential: () => Promise<HubResult<unknown>>;
  onClearVncCredential: () => Promise<HubResult<unknown>>;
  webInterfaces: WebInterfaceCalls;
}

/** The RDP and VNC passwords go through the sessions client, whose errors never quote the password (DEV-011, LOG-007). */
function saveCalls(hub: SettingsHub): DeviceSaveCalls {
  return {
    update: hub.devices.update,
    setSshCredential: hub.devices.setSshCredential,
    setRdpCredential: (id, credential) => hub.sessions.setCredential('RDP', id, credential.password),
    setVncCredential: (id, credential) => hub.sessions.setCredential('VNC', id, credential.password),
  };
}

/**
 * The Settings tab's calls for device `id` (WEB-001, WEB-002, DEV-011, SEC-007). `onSaved` runs after a save that
 * fully succeeded, so the form starts again from the saved device.
 */
export function settingsCalls(id: string, hub: SettingsHub, shown: ShownDevice, onSaved: () => void): SettingsCalls {
  async function patched<T>(pending: Promise<HubResult<T>>, change: (device: DeviceDetail, value: T) => DeviceDetail): Promise<HubResult<T>> {
    const result = await pending;
    if (result.ok && shown.device !== null) shown.setDevice(change(shown.device, result.value));
    return result;
  }

  return {
    onSave: async (values) => {
      if (shown.device === null) return { ok: true, value: undefined };
      const saved = await saveDevice(shown.device, values, saveCalls(hub));
      shown.setDevice(saved.device);
      if (saved.result.ok) onSaved();
      return saved.result;
    },
    onAcceptHostKey: (fingerprint) =>
      patched(hub.sessions.acceptHostKey(id, fingerprint), (device, hostKey) => ({ ...device, ssh: { ...device.ssh, hostKey } })),
    onAcceptServerCertificate: (fingerprint) => patched(hub.sessions.acceptServerCertificate(id, fingerprint), withServerCertificate),
    onClearCredential: () =>
      patched(hub.devices.clearSshCredential(id), (device, stored) => ({
        ...device,
        ssh: { ...device.ssh, credentialStored: stored.credentialStored },
      })),
    onClearRdpCredential: () => patched(hub.devices.clearRdpCredential(id), (device, stored) => withRdpStored(device, stored.credentialStored)),
    onClearVncCredential: () => patched(hub.devices.clearVncCredential(id), (device, stored) => withVncStored(device, stored.credentialStored)),
    webInterfaces: {
      onCreate: (input) => patched(hub.webInterfaces.create(id, input), withWebInterface),
      onUpdate: (webId, input) => patched(hub.webInterfaces.update(webId, input), withWebInterface),
      onDelete: (webId) => patched(hub.webInterfaces.delete(webId), withoutWebInterface),
      onAcceptSuggestion: (checkId) =>
        patched(hub.webInterfaces.acceptSuggestion(checkId), (device, web) => withWebInterface(withoutSuggestion(device, checkId), web)),
      onDismissSuggestion: (checkId) => patched(hub.webInterfaces.dismissSuggestion(checkId), withoutSuggestion),
    },
  };
}
