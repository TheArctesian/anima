// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { hubIsHosted, isHostedHub, markHosted, networkAdvice } from './mark';

const HUB = 'http://127.0.0.1:4810';

afterEach(() => {
  localStorage.clear();
});

describe('the mark of a hosted hub (R-129, R-211)', () => {
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
