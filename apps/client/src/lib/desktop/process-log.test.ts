import { beforeEach, describe, expect, it, vi } from 'vitest';
import { LogFolderError, openLogFolder, warnToProcessLog } from './process-log';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), warn: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke }));
vi.mock('@tauri-apps/plugin-log', () => ({ warn: shell.warn }));

beforeEach(() => {
  shell.invoke.mockReset();
  shell.warn.mockReset();
});

describe('openLogFolder', () => {
  it('LOG-008: invokes process_log_folder_open with no arguments', async () => {
    shell.invoke.mockResolvedValue(null);
    await openLogFolder();
    expect(shell.invoke).toHaveBeenCalledWith('process_log_folder_open');
  });

  it('LOG-008: the shell codes become typed errors without the shell message', async () => {
    shell.invoke.mockRejectedValue({ code: 'unavailable', message: '/Users/ann/secret path' });
    const failure = await openLogFolder().catch((reason: unknown) => reason);
    expect(failure).toBeInstanceOf(LogFolderError);
    expect(failure).toMatchObject({ problem: 'unavailable' });
    expect(String(failure)).not.toContain('secret');
    shell.invoke.mockRejectedValue('plugin not found');
    await expect(openLogFolder()).rejects.toMatchObject({ problem: 'failed' });
  });
});

describe('warnToProcessLog', () => {
  it('UPD-006: writes the given text through the log plugin', async () => {
    shell.warn.mockResolvedValue(undefined);
    warnToProcessLog('Update check failed.');
    await vi.waitFor(() => { expect(shell.warn).toHaveBeenCalledWith('Update check failed.'); });
  });

  it('UPD-006: a log plugin that rejects does not throw to the caller', async () => {
    shell.warn.mockRejectedValue(new Error('plugin not found'));
    const console_ = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    warnToProcessLog('Update check failed.');
    await vi.waitFor(() => { expect(console_).toHaveBeenCalledWith('The process log did not take a line:', 'Error'); });
    console_.mockRestore();
  });
});
