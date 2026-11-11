import { expect, it } from 'vitest';
import type { HubError } from '$lib/hub/hub';
import { fakeHub } from '../vpn.fixture';
import { enrolApi } from './api';

const KEY = { key: 'hskey-auth-abcdefghijkl-0123456789', expiresAt: '2026-11-10T13:00:00Z', steps: ['sudo tailscale up'] };

it('VPN-003: createInvite sends no arguments and returns the key, its expiry and the steps', async () => {
  const fake = fakeHub({ CreateVpnInvite: { createVpnInvite: KEY } });
  const result = await enrolApi(fake.hub).createInvite();
  expect(result).toEqual({ ok: true, value: KEY });
  expect(fake.requests).toEqual([{ operation: 'CreateVpnInvite', variables: {} }]);
});

it('VPN-001: createRouterKey sends the site id', async () => {
  const fake = fakeHub({ CreateRouterKey: { createRouterKey: KEY } });
  const result = await enrolApi(fake.hub).createRouterKey('site-home');
  expect(result).toEqual({ ok: true, value: KEY });
  expect(fake.requests).toEqual([{ operation: 'CreateRouterKey', variables: { siteId: 'site-home' } }]);
});

it('vpn.md 6B: register sends the auth ID and returns the registered node', async () => {
  const node = { name: 'mac', address: '100.64.0.9', keyExpiry: '2027-05-01T00:00:00Z' };
  const fake = fakeHub({ RegisterVpnNode: { registerVpnNode: node } });
  const result = await enrolApi(fake.hub).register('hskey-authreq-abc');
  expect(result).toEqual({ ok: true, value: node });
  expect(fake.requests[0]?.variables).toEqual({ authId: 'hskey-authreq-abc' });
});

it('VP-7: a hub refusal passes through as the typed error', async () => {
  const refusal: HubError = { code: 'CONFLICT', message: 'Too many keys in a short time.' };
  const fake = fakeHub({ CreateVpnInvite: refusal });
  expect(await enrolApi(fake.hub).createInvite()).toEqual({ ok: false, error: refusal });
});
