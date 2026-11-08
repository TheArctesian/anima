import { invoke } from '@tauri-apps/api/core';

/** Why the log folder did not open. `unavailable` and `failed` are the shell's codes (LOG-008). */
export type LogFolderProblem = 'unavailable' | 'failed';

const MESSAGES: Record<LogFolderProblem, string> = {
  unavailable: 'The log folder is not available.',
  failed: 'The log folder could not be opened.',
};

/** A failure to open the log folder. The message is fixed text; nothing from the shell's error is kept. */
export class LogFolderError extends Error {
  constructor(readonly problem: LogFolderProblem) {
    super(MESSAGES[problem]);
    this.name = 'LogFolderError';
  }
}

function problemOf(reason: unknown): LogFolderProblem {
  const code = typeof reason === 'object' && reason !== null && 'code' in reason ? reason.code : undefined;
  return code === 'unavailable' ? 'unavailable' : 'failed';
}

/** Opens the folder holding the process log in the system's file manager (LOG-008). */
export async function openLogFolder(): Promise<void> {
  try {
    await invoke('process_log_folder_open');
  } catch (reason) {
    throw new LogFolderError(problemOf(reason));
  }
}

/**
 * Writes one warning line to the process log. Callers pass fixed text only: the plugin writes whatever it is given to
 * a file on disk (LOG-007). A failure to write is reported on the console and never reaches the caller.
 */
export function warnToProcessLog(text: string): void {
  void import('@tauri-apps/plugin-log')
    .then(({ warn }) => warn(text))
    .catch((reason: unknown) => {
      console.warn('The process log did not take a line:', reason instanceof Error ? reason.name : 'unknown');
    });
}
