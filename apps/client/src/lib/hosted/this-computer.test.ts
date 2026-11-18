import { describe, expect, it, vi } from 'vitest';
import type { CloudDevice } from '$lib/desktop/cloud/device';
import type { HubError, HubResult } from '$lib/hub/hub';
import { addThisComputer, type ThisComputerParts } from './this-computer';

const ADDRESS = 'fd41:4e49:4d41::7';
const MAC: CloudDevice = { name: 'stephen-s-macbook-pro', computer: "Stephen's MacBook Pro", os: 'MACOS' };
const ON_LINK = [
  { name: 'desk', address: 'fd41:4e49:4d41::2' },
  { name: MAC.name, address: ADDRESS },
];
const LINK = { id: 'site-link', name: 'Link', subnet: 'fd41:4e49:4d41::/48' };

const ok = <T>(value: T): HubResult<T> => ({ ok: true, value });
const failed = (code: 'UNAVAILABLE' | 'CONFLICT' | 'FORBIDDEN' | 'unreachable' | 'malformed'): HubResult<never> => {
  const error: HubError = { code, message: 'x' };
  return { ok: false, error };
};

type Answers = { readonly [Part in keyof ThisComputerParts]?: Awaited<ReturnType<ThisComputerParts[Part]>> };

/** A hub with no device and no site, and this Mac on its link; `answers` replaces what a part answers. */
function parts(answers: Answers = {}) {
  return {
    device: vi.fn(() => Promise.resolve(answers.device === undefined ? MAC : answers.device)),
    linkDevices: vi.fn(() => Promise.resolve(answers.linkDevices ?? ok(ON_LINK))),
    hosts: vi.fn(() => Promise.resolve(answers.hosts ?? ok<readonly string[]>([]))),
    sites: vi.fn(() => Promise.resolve(answers.sites ?? ok<readonly (typeof LINK)[]>([]))),
    createSite: vi.fn<ThisComputerParts['createSite']>(() => Promise.resolve(answers.createSite ?? ok({ id: 'site-new' }))),
    createDevice: vi.fn<ThisComputerParts['createDevice']>(() => Promise.resolve(answers.createDevice ?? ok({}))),
  };
}

describe('addThisComputer (HMD-044, R-136)', () => {
  it('adds this computer at its link address, in a new site Link, with no check and no SSH', async () => {
    const hub = parts();
    expect(await addThisComputer(hub)).toBe(true);
    expect(hub.createSite).toHaveBeenCalledExactlyOnceWith('Link', 'fd41:4e49:4d41::/48');
    expect(hub.createDevice).toHaveBeenCalledExactlyOnceWith({
      siteId: 'site-new',
      name: "Stephen's MacBook Pro",
      host: ADDRESS,
      kind: 'workstation',
      os: 'MACOS',
      tags: [],
      addDefaultCheck: false,
    });
  });

  it('uses the site Link that the hub has', async () => {
    const hub = parts({ sites: ok([{ id: 'site-home', name: 'Home', subnet: '10.0.0.0/24' }, LINK]) });
    expect(await addThisComputer(hub)).toBe(true);
    expect(hub.createSite).not.toHaveBeenCalled();
    expect(hub.createDevice.mock.calls[0]?.[0].siteId).toBe('site-link');
  });

  it('uses a site with the link range that has a different name: the hub refuses a second site there', async () => {
    const hub = parts({ sites: ok([{ ...LINK, id: 'site-renamed', name: 'My computers' }]) });
    expect(await addThisComputer(hub)).toBe(true);
    expect(hub.createSite).not.toHaveBeenCalled();
    expect(hub.createDevice.mock.calls[0]?.[0].siteId).toBe('site-renamed');
  });

  it('a computer with no name gets its link name', async () => {
    const hub = parts({ device: { ...MAC, computer: '  ' } });
    expect(await addThisComputer(hub)).toBe(true);
    expect(hub.createDevice.mock.calls[0]?.[0].name).toBe('stephen-s-macbook-pro');
  });

});

describe('addThisComputer when there is nothing to add (HMD-044)', () => {
  it.each([
    ['the same text', ADDRESS],
    ['capital letters', 'FD41:4E49:4D41::7'],
    ['every group written', 'fd41:4e49:4d41:0:0:0:0:7'],
    ['leading zeros', 'fd41:4e49:4d41:0000:0000:0000:0000:0007'],
    ['brackets and spaces', ' [fd41:4e49:4d41::7] '],
  ])('does nothing when a device has the link address as its host, with %s', async (_what, host) => {
    const hub = parts({ hosts: ok(['10.0.0.5', 'nas.local', host]) });
    expect(await addThisComputer(hub)).toBe(false);
    expect(hub.sites).not.toHaveBeenCalled();
    expect(hub.createDevice).not.toHaveBeenCalled();
  });

  it('a device at a different link address is not this computer', async () => {
    const hub = parts({ hosts: ok(['fd41:4e49:4d41::70', 'fd41:4e49:4d41:7::']) });
    expect(await addThisComputer(hub)).toBe(true);
  });

  it('does nothing when the link has no device with the name of this computer', async () => {
    const hub = parts({ linkDevices: ok([{ name: 'desk', address: 'fd41:4e49:4d41::2' }]) });
    expect(await addThisComputer(hub)).toBe(false);
    expect(hub.hosts).not.toHaveBeenCalled();
    expect(hub.createDevice).not.toHaveBeenCalled();
  });

  it('does nothing when the shell cannot read this computer', async () => {
    const hub = parts({ device: null });
    expect(await addThisComputer(hub)).toBe(false);
    expect(hub.linkDevices).not.toHaveBeenCalled();
  });

  it.each([
    ['the link devices', { linkDevices: failed('UNAVAILABLE') }],
    ['the device hosts', { hosts: failed('malformed') }],
    ['the sites', { sites: failed('unreachable') }],
    ['the new site', { createSite: failed('CONFLICT') }],
  ] as const)('is quiet and adds nothing when the hub refuses %s', async (_what, answers) => {
    const hub = parts(answers);
    expect(await addThisComputer(hub)).toBe(false);
    expect(hub.createDevice).not.toHaveBeenCalled();
  });

  it('is quiet when the hub refuses the device, or a part throws', async () => {
    expect(await addThisComputer(parts({ createDevice: failed('FORBIDDEN') }))).toBe(false);
    const broken = parts();
    broken.hosts.mockRejectedValue(new Error('gone'));
    expect(await addThisComputer(broken)).toBe(false);
    expect(broken.createDevice).not.toHaveBeenCalled();
  });

  it('two pages that open at the same time add one device', async () => {
    const hub = parts();
    expect(await Promise.all([addThisComputer(hub), addThisComputer(hub)])).toEqual([true, true]);
    expect(hub.createDevice).toHaveBeenCalledOnce();
  });
});
