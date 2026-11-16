import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  createLocalAccount,
  installLocalHub,
  localHubCarried,
  localHubMessage,
  newTotp,
  readLocalAccount,
  readLocalHub,
  saveHubKey,
  startLocalHub,
  statusOf,
  stopLocalHub,
  type LocalHubProblem,
} from './shell';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));

const READY = { installed: true, running: true, state: 'ready', hub: '127.0.0.1:8443', recovered: false, link: { state: 'up', since: '2026-11-15T10:00:00Z' } };
const consoleSpies = (['log', 'info', 'warn', 'error', 'debug'] as const).map((name) => vi.spyOn(console, name).mockImplementation(() => undefined));

beforeEach(() => {
  shell.invoke.mockReset().mockResolvedValue(null);
  shell.isTauri.mockReturnValue(true);
  for (const spy of consoleSpies) spy.mockClear();
});

describe('statusOf', () => {
  it('FRN-006, LHB-010: takes each state the shell reports, with only the fields it sent', () => {
    expect(statusOf(READY)).toEqual(READY);
    expect(statusOf({ installed: false, running: false })).toEqual({ installed: false, running: false });
    expect(statusOf({ installed: true, running: true, state: 'blocked', reason: 'key-missing' })).toEqual({ installed: true, running: true, state: 'blocked', reason: 'key-missing' });
    expect(statusOf({ installed: true, running: false, state: 'failed', cause: 'anima-gateway', hub: null })).toEqual({ installed: true, running: false, state: 'failed', cause: 'anima-gateway' });
  });

  it('refuses anything that is not a status, so a wrong answer never shows as a state', () => {
    const wrong = [null, 'ready', {}, { installed: 'yes', running: true }, { ...READY, state: 'up' }, { ...READY, reason: 'other' }, { ...READY, link: { state: 'sideways', since: '' } }, { ...READY, hub: 8443 }];
    for (const each of wrong) expect(statusOf(each)).toBeNull();
  });
});

describe('the shell calls', () => {
  it('FRN-002, FRN-006: each call names its command and passes only its arguments', async () => {
    shell.invoke.mockImplementation((command: string) => Promise.resolve(command === 'local_hub_status' ? READY : command === 'local_hub_totp_new' ? { secret: 'S', uri: 'otpauth://totp/x' } : null));
    await installLocalHub();
    await readLocalHub();
    await startLocalHub();
    await stopLocalHub();
    await readLocalAccount();
    await newTotp('ann');
    await createLocalAccount({ name: 'ann', password: 'p', secret: 's', code: '123456' });
    expect(shell.invoke.mock.calls).toEqual([
      ['local_hub_install', undefined],
      ['local_hub_status', undefined],
      ['local_hub_start', undefined],
      ['local_hub_stop', undefined],
      ['local_hub_account', undefined],
      ['local_hub_totp_new', { name: 'ann' }],
      ['local_hub_account_create', { account: { name: 'ann', password: 'p', secret: 's', code: '123456' } }],
    ]);
  });

  it('FRN-003: the account is null before the first run, and its name and TOTP after', async () => {
    expect(await readLocalAccount()).toEqual({ ok: true, value: null });
    shell.invoke.mockResolvedValue({ name: 'ann', totp: true });
    expect(await readLocalAccount()).toEqual({ ok: true, value: { name: 'ann', totp: true } });
    shell.invoke.mockResolvedValue({ name: 'ann' });
    expect((await readLocalAccount()).ok).toBe(false);
  });

  it('FRN-004: a TOTP answer must carry an otpauth URI', async () => {
    shell.invoke.mockResolvedValue({ secret: 'S', uri: 'https://evil.example' });
    expect(await newTotp('ann')).toMatchObject({ ok: false, problem: 'failed' });
  });

  it('2.8: the key copy answers yes, or no when the operator cancelled', async () => {
    shell.invoke.mockResolvedValue(false);
    expect(await saveHubKey()).toEqual({ ok: true, value: false });
    shell.invoke.mockResolvedValue(true);
    expect(await saveHubKey()).toEqual({ ok: true, value: true });
    expect(shell.invoke.mock.calls.map(([command]) => command as string)).toEqual(['local_hub_key_save', 'local_hub_key_save']);
  });

  it('FRN-007, F-9: a recovery the person declined in the native dialog says nothing changed', async () => {
    shell.invoke.mockRejectedValue({ code: 'cancelled', message: 'x' });
    expect(await createLocalAccount({ name: 'ann', password: 'p', secret: 's', code: '1' })).toEqual({ ok: false, problem: 'cancelled', message: 'Recovery was cancelled. Nothing changed.' });
  });

});

describe('the refusals', () => {
  it("LOG-007: a refusal keeps the shell's code and a fixed message, never the shell's text", async () => {
    shell.invoke.mockRejectedValue({ code: 'refused', message: 'raw hub text with hunter2' });
    const result = await createLocalAccount({ name: 'ann', password: 'hunter2', secret: 's', code: '1' });
    expect(result).toEqual({ ok: false, problem: 'refused', message: localHubMessage('refused') });
    expect(JSON.stringify(result)).not.toContain('hunter2');
    shell.invoke.mockRejectedValue('command not found');
    expect(await installLocalHub()).toMatchObject({ ok: false, problem: 'failed' });
    for (const spy of consoleSpies) expect(spy).not.toHaveBeenCalled();
  });

  it('FRN-001: carried is true only when the shell says so, and false outside the desktop app', async () => {
    shell.invoke.mockResolvedValue(true);
    expect(await localHubCarried()).toBe(true);
    shell.invoke.mockRejectedValue({ code: 'caller' });
    expect(await localHubCarried()).toBe(false);
    shell.isTauri.mockReturnValue(false);
    shell.invoke.mockResolvedValue(true);
    expect(await localHubCarried()).toBe(false);
    expect(await readLocalHub()).toMatchObject({ ok: false, problem: 'unavailable' });
  });

  it('every code has its own message', () => {
    const codes: LocalHubProblem[] = ['caller', 'unavailable', 'not-carried', 'busy', 'no-home', 'timed-out', 'failed', 'argument', 'refused', 'no-key', 'destination', 'cancelled'];
    expect(new Set(codes.map(localHubMessage)).size).toBe(codes.length);
  });
});
