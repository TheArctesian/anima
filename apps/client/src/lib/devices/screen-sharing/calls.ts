import { openRemoteLoginSettings, remoteLoginSettingsAvailable } from '$lib/desktop/systemSettings';
import { readThisMachine } from '$lib/desktop/thisMachine';
import { devicesApi } from '$lib/devices/client';
import type { Hub } from '$lib/hub/hub';
import { sessionsApi } from '$lib/sessions/api';
import { RETEST_INTERVAL_MS, type ScreenSharingCalls } from './setup';

/** The calls of the screen sharing flow, on the hub and the desktop shell (R-079). */
export function screenSharingCalls(hub: Hub): ScreenSharingCalls {
  return {
    test: sessionsApi(hub).testConnection,
    update: devicesApi(hub).update,
    thisMachine: readThisMachine,
    sharingAvailable: remoteLoginSettingsAvailable,
    openSharing: openRemoteLoginSettings,
    after: (ms, run) => {
      const timer = setTimeout(run, ms);
      return () => {
        clearTimeout(timer);
      };
    },
    intervalMs: RETEST_INTERVAL_MS,
  };
}
