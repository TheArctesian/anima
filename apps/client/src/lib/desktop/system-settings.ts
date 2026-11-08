import { invoke } from '@tauri-apps/api/core';

/** Why System Settings did not open. `unavailable` and `failed` are the shell's codes (R-059). */
export type SystemSettingsProblem = 'unavailable' | 'failed';

const MESSAGES: Record<SystemSettingsProblem, string> = {
  unavailable: 'Sharing settings cannot be opened on this computer.',
  failed: 'System Settings did not open.',
};

/** A failure to open System Settings. The message is fixed text; nothing from the shell's error is kept. */
export class SystemSettingsError extends Error {
  constructor(readonly problem: SystemSettingsProblem) {
    super(MESSAGES[problem]);
    this.name = 'SystemSettingsError';
  }
}

/** Opens System Settings at General → Sharing, where Remote Login (R-059) and Screen Sharing (R-079) are turned on. */
export async function openSharingSettings(): Promise<void> {
  try {
    await invoke('sharing_settings_open');
  } catch (reason) {
    const code = typeof reason === 'object' && reason !== null && 'code' in reason ? reason.code : undefined;
    throw new SystemSettingsError(code === 'unavailable' ? 'unavailable' : 'failed');
  }
}
