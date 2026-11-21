import { beforeEach, describe, expect, it, vi } from 'vitest';
import { GATEWAY_PATH, readAccountUsage } from './read';
import { POOL_ANSWER } from './usage.fixture';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => false) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));

const replying = (status: number, body: unknown = {}) =>
  vi.fn<typeof fetch>(() => Promise.resolve(new Response(JSON.stringify(body), { status })));

beforeEach(() => {
  shell.invoke.mockReset();
  shell.isTauri.mockReturnValue(false);
});

describe('reading the figures of the account (HMD-073 in form B)', () => {
  it('in a browser asks the path of the gateway on the origin of the hub, with its session', async () => {
    const fetcher = replying(200, POOL_ANSWER);
    const answer = await readAccountUsage(false, fetcher);
    expect(answer.state).toBe('figures');
    expect(fetcher).toHaveBeenCalledWith(GATEWAY_PATH, expect.objectContaining({ credentials: 'same-origin' }));
  });

  it('in a browser: 401 is no sign-in of the account; any other failure is not available', async () => {
    expect(await readAccountUsage(false, replying(401))).toEqual({ state: 'signed-out' });
    expect(await readAccountUsage(false, replying(503))).toEqual({ state: 'unavailable' });
    expect(await readAccountUsage(false, replying(200, { at: 'no' }))).toEqual({ state: 'unavailable' });
    expect(await readAccountUsage(false, vi.fn<typeof fetch>(() => Promise.reject(new TypeError('offline'))))).toEqual({ state: 'unavailable' });
  });

  it('in the application asks the shell, which calls the route of the site with the sign-in it holds', async () => {
    shell.isTauri.mockReturnValue(true);
    shell.invoke.mockResolvedValueOnce(POOL_ANSWER);
    const fetcher = replying(200, POOL_ANSWER);
    expect((await readAccountUsage(true, fetcher)).state).toBe('figures');
    expect(shell.invoke).toHaveBeenCalledWith('cloud_hub_usage', undefined);
    expect(fetcher).not.toHaveBeenCalled();
    shell.invoke.mockRejectedValueOnce({ code: 'unauthenticated' });
    expect(await readAccountUsage(true, fetcher)).toEqual({ state: 'signed-out' });
    for (const code of ['offline', 'not-ready', 'unconfigured']) {
      shell.invoke.mockRejectedValueOnce({ code });
      expect(await readAccountUsage(true, fetcher)).toEqual({ state: 'unavailable' });
    }
  });
});
