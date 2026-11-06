import { invoke, isTauri } from '@tauri-apps/api/core';

/** Why System Settings did not open. `unavailable` and `failed` are the shell's codes (R-059). */
export type SystemSettingsProblem = 'unavailable' | 'failed';

const MESSAGES: Record<SystemSettingsProblem, string> = {
  unavailable: 'Remote Login settings cannot be opened on this computer.',
  failed: 'System Settings did not open.',
};

/** A failure to open System Settings. The message is fixed text; nothing from the shell's error is kept. */
export class SystemSettingsError extends Error {
  constructor(readonly problem: SystemSettingsProblem) {
    super(MESSAGES[problem]);
    this.name = 'SystemSettingsError';
  }
}

/** Whether this is the desktop app on a Mac, where Remote Login (SSH) is a switch in System Settings (R-059). */
export function remoteLoginSettingsAvailable(userAgent: string = navigator.userAgent): boolean {
  return isTauri() && /Macintosh|Mac OS X/.test(userAgent);
}

/** Opens System Settings at General → Sharing, where Remote Login is turned on (R-059). */
export async function openRemoteLoginSettings(): Promise<void> {
  try {
    await invoke('remote_login_settings_open');
  } catch (reason) {
    const code = typeof reason === 'object' && reason !== null && 'code' in reason ? reason.code : undefined;
    throw new SystemSettingsError(code === 'unavailable' ? 'unavailable' : 'failed');
  }
}
