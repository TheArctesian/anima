import { beforeEach, describe, expect, it, vi } from 'vitest';
import { desktopWebView, fingerprintFrom, WebViewError, type Fingerprint } from './webView';

const shell = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke, isTauri: shell.isTauri }));
vi.mock('@tauri-apps/api/event', () => ({ listen: shell.listen }));

const PIN = fingerprintFrom('ab'.repeat(32)) as Fingerprint;
const BOUNDS = { x: 0, y: 48, width: 800, height: 552 };

beforeEach(() => {
  shell.invoke.mockReset();
  shell.invoke.mockResolvedValue(null);
  shell.isTauri.mockReturnValue(true);
  shell.listen.mockReset();
});

describe('desktopWebView commands (WP-31 shell interface)', () => {
  it('WEB-004: sends each command with the arguments of the interface table', async () => {
    await desktopWebView.open({ tabId: 't-1', deviceId: 'd-1', url: 'https://nas.lan/', pin: PIN, bounds: BOUNDS });
    await desktopWebView.place('t-1', BOUNDS);
    await desktopWebView.show('t-1', false);
    await desktopWebView.go('t-1', 'BACK');
    await desktopWebView.pin('t-1', PIN);
    await desktopWebView.popOut('t-1');
    await desktopWebView.close('t-1');
    await desktopWebView.openSystemBrowser('http://printer.lan/');
    expect(shell.invoke.mock.calls).toEqual([
      ['web_view_open', { tabId: 't-1', deviceId: 'd-1', url: 'https://nas.lan/', pin: PIN, bounds: BOUNDS }],
      ['web_view_place', { tabId: 't-1', bounds: BOUNDS }],
      ['web_view_show', { tabId: 't-1', shown: false }],
      ['web_view_go', { tabId: 't-1', to: 'BACK' }],
      ['web_view_pin', { tabId: 't-1', pin: PIN }],
      ['web_view_pop_out', { tabId: 't-1' }],
      ['web_view_close', { tabId: 't-1' }],
      ['system_browser_open', { url: 'http://printer.lan/' }],
    ]);
  });

  it('LOG-007: a refusal keeps only a well-formed code, never the shell text', async () => {
    shell.invoke.mockRejectedValue({ code: 'not_found', message: 'https://nas.lan/secret?token=1' });
    const failure = await desktopWebView.close('t-1').catch((reason: unknown) => reason);
    expect(failure).toBeInstanceOf(WebViewError);
    expect(failure).toMatchObject({ code: 'not_found' });
    expect(String(failure)).not.toContain('nas.lan');
    shell.invoke.mockRejectedValue({ code: 'https://nas.lan/' });
    await expect(desktopWebView.close('t-1')).rejects.toMatchObject({ code: 'failed' });
    shell.invoke.mockRejectedValue('raw text');
    await expect(desktopWebView.close('t-1')).rejects.toMatchObject({ code: 'failed' });
  });

  it('D-6: pinning is true only when the shell answers exactly true', async () => {
    shell.invoke.mockResolvedValue(true);
    expect(await desktopWebView.pinning()).toBe(true);
    for (const answer of ['true', 1, null, { pinning: true }]) {
      shell.invoke.mockResolvedValue(answer);
      expect(await desktopWebView.pinning()).toBe(false);
    }
    shell.invoke.mockRejectedValue({ code: 'failed' });
    expect(await desktopWebView.pinning()).toBe(false);
    shell.isTauri.mockReturnValue(false);
    shell.invoke.mockResolvedValue(true);
    expect(await desktopWebView.pinning()).toBe(false);
  });

  it('WEB-008: reads a certificate fingerprint and refuses any other answer', async () => {
    shell.invoke.mockResolvedValue({ fingerprintSha256: PIN });
    expect(await desktopWebView.readCertificate('https://nas.lan/')).toBe(PIN);
    expect(shell.invoke).toHaveBeenCalledWith('web_certificate_read', { url: 'https://nas.lan/' });
    for (const answer of [{ fingerprintSha256: PIN.toUpperCase() }, { fingerprint: PIN }, PIN, null]) {
      shell.invoke.mockResolvedValue(answer);
      await expect(desktopWebView.readCertificate('https://nas.lan/')).rejects.toMatchObject({ code: 'malformed' });
    }
  });

});

describe('desktopWebView URL guard', () => {
  it('never sends a URL the shell must not take: only http and https open, only https is read', async () => {
    await expect(desktopWebView.openSystemBrowser('file:///etc/passwd')).rejects.toMatchObject({ code: 'invalid_url' });
    await expect(desktopWebView.openSystemBrowser('javascript:alert(1)')).rejects.toMatchObject({ code: 'invalid_url' });
    await expect(desktopWebView.readCertificate('http://nas.lan/')).rejects.toMatchObject({ code: 'invalid_url' });
    expect(shell.invoke).not.toHaveBeenCalled();
  });
});

describe('desktopWebView.listen', () => {
  it('hears web-view on the main web view only and passes on checked events only', async () => {
    const stop = vi.fn();
    shell.listen.mockResolvedValue(stop);
    const heard: unknown[] = [];
    expect(await desktopWebView.listen((event) => heard.push(event))).toBe(stop);
    expect(shell.listen).toHaveBeenCalledWith('web-view', expect.any(Function), { target: { kind: 'WebviewWindow', label: 'main' } });
    const deliver = shell.listen.mock.calls[0]?.[1] as (event: { event: string; payload: unknown }) => void;
    deliver({ event: 'web-view', payload: { tabId: 't-1', kind: 'LOADED', url: 'http://a/' } });
    deliver({ event: 'web-view', payload: { tabId: 't-1', kind: 'LOADED', url: 'file:///x' } });
    deliver({ event: 'other', payload: { tabId: 't-1', kind: 'LOADED', url: 'http://b/' } });
    expect(heard).toEqual([{ tabId: 't-1', kind: 'LOADED', url: 'http://a/' }]);
  });
});
