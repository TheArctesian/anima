import { describe, expect, it, vi } from 'vitest';
import { openWebInterface, type WebOpening } from './open';

const device = { id: 'd1', name: 'nas', status: 'DOWN' } as const;
const origin = { left: 1, top: 2, width: 3, height: 4 };

function opening(url: string, openIn: WebOpening['webInterface']['openIn']): WebOpening {
  return { device, origin, webInterface: { id: 'w1', deviceId: 'd1', url, label: 'Admin', openIn, pinnedFingerprintSha256: 'ab'.repeat(32) } };
}

function ports(browser: () => Promise<void> = () => Promise.resolve()) {
  return { sessions: { openWeb: vi.fn(() => 'tab-1') }, shell: { openSystemBrowser: vi.fn(browser) } };
}

describe('openWebInterface (WEB-003, WEB-011)', () => {
  it('WEB-003 / MOT-005: ANIMA_TAB opens a web tab for the device with its status, from the button', async () => {
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening('http://nas.lan/', 'ANIMA_TAB'));
    expect(shell.openSystemBrowser).not.toHaveBeenCalled();
    expect(sessions.openWeb).toHaveBeenCalledExactlyOnceWith({
      deviceId: 'd1',
      deviceName: 'nas',
      status: 'DOWN',
      web: { id: 'w1', deviceId: 'd1', url: 'http://nas.lan/', label: 'Admin', pinnedFingerprintSha256: 'ab'.repeat(32) },
      origin,
    });
  });

  it('WEB-011: SYSTEM_BROWSER over http opens the system browser and no tab', async () => {
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening('http://nas.lan/', 'SYSTEM_BROWSER'));
    expect(shell.openSystemBrowser).toHaveBeenCalledExactlyOnceWith('http://nas.lan/');
    expect(sessions.openWeb).not.toHaveBeenCalled();
  });

  it('WEB-008 / WEB-011: SYSTEM_BROWSER over https opens a web tab, which checks the pin', async () => {
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening('https://nas.lan/', 'SYSTEM_BROWSER'));
    expect(shell.openSystemBrowser).not.toHaveBeenCalled();
    expect(sessions.openWeb).toHaveBeenCalledOnce();
  });

  it('WEB-011: a system browser that cannot be opened leaves the web tab to say why', async () => {
    const { sessions, shell } = ports(() => Promise.reject(new Error('no browser')));
    await openWebInterface(sessions, shell, opening('http://nas.lan/', 'SYSTEM_BROWSER'));
    expect(sessions.openWeb).toHaveBeenCalledOnce();
  });
});
