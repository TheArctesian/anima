import { beforeEach, expect, it, vi } from 'vitest';
import { checkHubListener } from './shell';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));

beforeEach(() => {
  shell.invoke.mockReset();
  shell.isTauri.mockReturnValue(true);
});

it.each(['own', 'other', 'none'] as const)('F-10: the shell answer %s is the listener', async (listener) => {
  shell.invoke.mockResolvedValue(listener);
  expect(await checkHubListener('http://127.0.0.1:8443')).toEqual({ ok: true, listener });
  expect(shell.invoke).toHaveBeenCalledWith('hub_listener_check', { origin: 'http://127.0.0.1:8443' });
});

it('F-10: any other answer, or an error, is no answer', async () => {
  shell.invoke.mockResolvedValue('mine');
  expect(await checkHubListener('http://127.0.0.1:8443')).toEqual({ ok: false, problem: 'failed' });
  shell.invoke.mockRejectedValue({ code: 'bad-origin', message: 'x' });
  expect(await checkHubListener('http://example.org')).toEqual({ ok: false, problem: 'bad-origin' });
  shell.invoke.mockRejectedValue('gone');
  expect(await checkHubListener('http://127.0.0.1:8443')).toEqual({ ok: false, problem: 'failed' });
});

it('CLI-001: outside the desktop app there is no check', async () => {
  shell.isTauri.mockReturnValue(false);
  expect(await checkHubListener('http://127.0.0.1:8443')).toEqual({ ok: false, problem: 'unavailable' });
  expect(shell.invoke).not.toHaveBeenCalled();
});
