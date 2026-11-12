import { beforeEach, describe, expect, it, vi } from 'vitest';
import { desktopUpdater, failureOf, UpdaterError } from './updater';

const plugins = vi.hoisted(() => ({
  check: vi.fn(),
  relaunch: vi.fn(),
  getVersion: vi.fn(),
}));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: plugins.check }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: plugins.relaunch }));
vi.mock('@tauri-apps/api/app', () => ({ getVersion: plugins.getVersion }));

const SECRET = 'https://user:pw@example.test/latest.json';

function update(download: () => Promise<void> = () => Promise.resolve()) {
  return { version: '1.2.0', downloadAndInstall: vi.fn(download), close: vi.fn(() => Promise.resolve()) };
}

beforeEach(() => {
  Object.values(plugins).forEach((mock) => mock.mockReset());
});

describe('desktopUpdater', () => {
  it('UPD-001: check calls the plugin with no options, so allowDowngrades stays in the shell configuration', async () => {
    plugins.check.mockResolvedValue(null);
    await expect(desktopUpdater.check()).resolves.toBeNull();
    expect(plugins.check).toHaveBeenCalledWith();
  });

  it('UPD-003: an offered update carries its version and installs through downloadAndInstall, then closes', async () => {
    const found = update();
    plugins.check.mockResolvedValue(found);
    const offered = await desktopUpdater.check();
    expect(offered?.version).toBe('1.2.0');
    await offered?.install();
    expect(found.downloadAndInstall).toHaveBeenCalledOnce();
    expect(found.close).toHaveBeenCalledOnce();
  });

  it('UPD-002: a failed install (bad signature) rejects with a fixed-text error and closes the update', async () => {
    const found = update(() => Promise.reject(new Error(`signature failed for ${SECRET}`)));
    plugins.check.mockResolvedValue(found);
    const failure = await (await desktopUpdater.check())?.install().catch((reason: unknown) => reason);
    expect(failure).toBeInstanceOf(UpdaterError);
    expect(failure).toMatchObject({ step: 'install', message: 'The update could not be installed.' });
    expect((failure as Error).cause).toBeUndefined();
    expect(found.close).toHaveBeenCalledOnce();
  });

  it('LOG-007: a failed check rejects with fixed text and no cause', async () => {
    plugins.check.mockRejectedValue(new Error(SECRET));
    const failure = await desktopUpdater.check().catch((reason: unknown) => reason);
    expect(failure).toMatchObject({ step: 'check', message: 'The update feed could not be checked.' });
    expect(JSON.stringify(failure)).not.toContain('pw');
    expect((failure as Error).cause).toBeUndefined();
  });
});

describe('desktopUpdater failure reasons', () => {
  it('UPD-002: the plugin refusing a signature, or the version it was signed for, is sorted as a signature failure', () => {
    expect(failureOf('The signature verification failed')).toBe('signature');
    expect(failureOf(new Error('The update was signed for version 1.0.0 but the update endpoint announced version 2.0.0.'))).toBe('signature');
    expect(failureOf('The signature was created with a different key than the one provided')).toBe('signature');
  });

  it('BLD-068: a 404 feed and a feed with no entry for this platform are sorted as no feed; the network is other', () => {
    expect(failureOf('Could not fetch a valid release JSON from the remote')).toBe('no-feed');
    expect(failureOf('the platform `darwin-aarch64` was not found in the response `platforms` object')).toBe('no-feed');
    expect(failureOf('error sending request for url (https://releases.anima.ing/linux-x86_64/latest.json)')).toBe('other');
    expect(failureOf(undefined)).toBe('other');
  });

  it('UPD-002: a failed install through the shell carries the signature reason and still keeps no cause', async () => {
    plugins.check.mockResolvedValue(update(() => Promise.reject('The signature verification failed')));
    const failure = await (await desktopUpdater.check())?.install().catch((reason: unknown) => reason);
    expect(failure).toMatchObject({ step: 'install', reason: 'signature' });
    expect((failure as Error).cause).toBeUndefined();
  });

  it('BLD-068: a failed check on a missing feed carries the no-feed reason', async () => {
    plugins.check.mockRejectedValue('Could not fetch a valid release JSON from the remote');
    await expect(desktopUpdater.check()).rejects.toMatchObject({ step: 'check', reason: 'no-feed' });
  });

  it('UPD-003: restart calls relaunch; a failure is a typed error', async () => {
    plugins.relaunch.mockResolvedValue(undefined);
    await desktopUpdater.restart();
    expect(plugins.relaunch).toHaveBeenCalledOnce();
    plugins.relaunch.mockRejectedValue(new Error('x'));
    await expect(desktopUpdater.restart()).rejects.toMatchObject({ step: 'restart' });
  });

  it('UPD-006: currentVersion is the app version', async () => {
    plugins.getVersion.mockResolvedValue('1.0.0');
    await expect(desktopUpdater.currentVersion()).resolves.toBe('1.0.0');
  });
});
