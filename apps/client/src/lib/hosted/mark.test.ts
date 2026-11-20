// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { noAnswer, UNREACHABLE } from '$lib/hub/hub';
import { HOSTED_ADDRESS_MOVED, HOSTED_BROWSER_ORIGIN, holdsMovedAddress, hubIsHosted, isHostedHub, markHosted, networkAdvice } from './mark';

const HUB = 'http://127.0.0.1:4810';

afterEach(() => {
  localStorage.clear();
});

describe('the mark of a hosted hub (R-129, R-222)', () => {
  it('a hub is hosted only at the address that was marked', () => {
    expect(hubIsHosted(HUB)).toBe(false);
    markHosted(localStorage, HUB);
    expect(hubIsHosted(HUB)).toBe(true);
    expect(hubIsHosted('https://hub.example.net')).toBe(false);
  });

  it('a storage that refuses to be read gives a hub that is not hosted', () => {
    const refusing = { getItem: () => { throw new Error('refused'); } } as unknown as Storage;
    expect(isHostedHub(refusing, HUB)).toBe(false);
  });
});

describe('NET-004: the advice when the hub does not answer', () => {
  it('names the network of the computer for a hosted hub, and never a VPN', () => {
    expect(networkAdvice(true)).toBe("Check this computer's network");
  });

  it('names the VPN client for each other hub', () => {
    expect(networkAdvice(false)).toBe('Check the VPN client');
  });
});

describe('the address that moved (HM-T31)', () => {
  it('an application that holds hub.anima.ing as its hub has the hub of before, and a browser page of that address has not', () => {
    expect(holdsMovedAddress('tauri://localhost', HOSTED_BROWSER_ORIGIN)).toBe(true);
    expect(holdsMovedAddress(null, HOSTED_BROWSER_ORIGIN)).toBe(true);
    expect(holdsMovedAddress(HOSTED_BROWSER_ORIGIN, HOSTED_BROWSER_ORIGIN)).toBe(false);
    expect(holdsMovedAddress('tauri://localhost', 'https://hub.example.net')).toBe(false);
    expect(holdsMovedAddress('tauri://localhost', 'http://127.0.0.1:62037')).toBe(false);
  });

  it('a call of the application to that address that gets no answer says that the address moved, and any other address does not', () => {
    // The page of this test is not served by hub.anima.ing, as the page of the application is not.
    expect(noAnswer(HOSTED_BROWSER_ORIGIN)).toEqual({ code: 'unreachable', message: HOSTED_ADDRESS_MOVED });
    expect(noAnswer('https://hub.example.net')).toBe(UNREACHABLE);
  });

  it('the words name the account and the address of an own hub, and no VPN', () => {
    expect(HOSTED_ADDRESS_MOVED).toContain('Anima.ing account');
    expect(HOSTED_ADDRESS_MOVED).toContain('address of your own hub');
    expect(HOSTED_ADDRESS_MOVED).not.toContain('VPN');
  });
});
