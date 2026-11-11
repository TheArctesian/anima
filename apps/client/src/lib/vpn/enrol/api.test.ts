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

it('BD-17: renew sends the node name and returns the new expiry', async () => {
  const node = { name: 'mac', address: '100.64.0.9', keyExpiry: '2027-03-01T00:00:00Z' };
  const fake = fakeHub({ RenewVpnKey: { renewVpnKey: node } });
  expect(await enrolApi(fake.hub).renew('mac')).toEqual({ ok: true, value: node });
  expect(fake.requests).toEqual([{ operation: 'RenewVpnKey', variables: { name: 'mac' } }]);
});

it('R-055: approve sends the site id and is true when the hub accepts', async () => {
  const fake = fakeHub({ ApproveSiteRoute: { approveSiteRoute: true } });
  expect(await enrolApi(fake.hub).approve('site-home')).toEqual({ ok: true, value: true });
  expect(fake.requests).toEqual([{ operation: 'ApproveSiteRoute', variables: { siteId: 'site-home' } }]);
});

it('R-055: a false answer is an error, not an approval', async () => {
  const fake = fakeHub({ ApproveSiteRoute: { approveSiteRoute: false } });
  const result = await enrolApi(fake.hub).approve('site-home');
  expect(result.ok).toBe(false);
});

it('R-055: a not-allowed refusal passes through with its details', async () => {
  const refusal: HubError = { code: 'VALIDATION', message: 'Another router serves this subnet.', fields: [{ field: 'siteId', message: 'Another router serves this subnet.' }] };
  const fake = fakeHub({ ApproveSiteRoute: refusal });
  expect(await enrolApi(fake.hub).approve('site-home')).toEqual({ ok: false, error: refusal });
});
