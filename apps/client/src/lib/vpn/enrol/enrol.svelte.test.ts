import { expect, it } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import type { VpnKey } from './api';
import { Enrolment, type EnrolCalls, type EnrolEnv } from './enrol.svelte';

const KEY: VpnKey = { key: 'hskey-auth-abcdefghijkl-0123456789', expiresAt: '2026-11-10T13:00:00Z', steps: ['sudo tailscale up'] };

function setup(change: Partial<EnrolCalls> = {}, clock = { at: new Date('2026-11-10T12:00:00Z') }) {
  const copied: string[] = [];
  const env: EnrolEnv = {
    now: () => clock.at,
    clipboard: { writeText: (text) => { copied.push(text); return Promise.resolve(); } },
    later: () => () => undefined,
  };
  const api: EnrolCalls = {
    createInvite: () => Promise.resolve({ ok: true, value: KEY }),
    createRouterKey: () => Promise.resolve({ ok: true, value: KEY }),
    register: () => Promise.resolve({ ok: true, value: { name: 'mac', address: '100.64.0.9', keyExpiry: null } }),
    ...change,
  };
  return { enrolment: new Enrolment(api, env), clock, copied };
}

it('VPN-003: an invite shows the key view with the key and counts down from the hub expiry', async () => {
  const { enrolment } = setup();
  await enrolment.makeInvite();
  expect(enrolment.view).toBe('key');
  expect(enrolment.key).toEqual(KEY);
  expect(enrolment.left).toBe(3600);
});

it('VPN-001: a router key is asked for the chosen site', async () => {
  const asked: string[] = [];
  const { enrolment } = setup({ createRouterKey: (siteId) => { asked.push(siteId); return Promise.resolve({ ok: true, value: KEY }); } });
  await enrolment.makeRouterKey('site-home');
  expect(asked).toEqual(['site-home']);
});

it('VP-7: the key is cleared when the modal closes', async () => {
  const { enrolment } = setup();
  await enrolment.makeInvite();
  enrolment.close();
  expect(enrolment.key).toBeNull();
  expect(enrolment.view).toBe('choose');
  expect(enrolment.left).toBe(0);
});

it('VP-7: the key is cleared when the step changes', async () => {
  const { enrolment } = setup();
  await enrolment.makeInvite();
  enrolment.openRegister();
  expect(enrolment.key).toBeNull();
  expect(enrolment.view).toBe('register');
  await enrolment.makeInvite();
  enrolment.choose();
  expect(enrolment.key).toBeNull();
});

it('VP-7: the key is cleared when it expires, and the operator is told', async () => {
  const { enrolment, clock } = setup();
  await enrolment.makeInvite();
  clock.at = new Date('2026-11-10T13:00:01Z');
  enrolment.tick();
  expect(enrolment.key).toBeNull();
  expect(enrolment.expired).toBe(true);
  expect(enrolment.view).toBe('choose');
});

it('VP-7: a key that arrives after the modal closed does not come back', async () => {
  let answer: (result: HubResult<VpnKey>) => void = () => undefined;
  const { enrolment } = setup({ createInvite: () => new Promise((resolve) => { answer = resolve; }) });
  const pending = enrolment.makeInvite();
  enrolment.close();
  answer({ ok: true, value: KEY });
  await pending;
  expect(enrolment.key).toBeNull();
  expect(enrolment.view).toBe('choose');
});

it('VP-7: a used router key leaves the page', async () => {
  const { enrolment } = setup();
  await enrolment.makeRouterKey('site-home');
  enrolment.keyUsed();
  expect(enrolment.key).toBeNull();
});

it('VP-7: a refusal stays on the choice with the hub error and makes no key', async () => {
  const error = { code: 'CONFLICT', message: 'Too many keys in a short time.' } as const;
  const { enrolment } = setup({ createInvite: () => Promise.resolve({ ok: false, error }) });
  await enrolment.makeInvite();
  expect(enrolment.error).toEqual(error);
  expect(enrolment.key).toBeNull();
  expect(enrolment.view).toBe('choose');
  expect(enrolment.busy).toBe(false);
});

it('vpn.md 6B: register trims the auth ID and shows the registered computer', async () => {
  const sent: string[] = [];
  const { enrolment } = setup({ register: (authId) => { sent.push(authId); return Promise.resolve({ ok: true, value: { name: 'mac', address: '100.64.0.9', keyExpiry: null } }); } });
  enrolment.openRegister();
  await enrolment.register('  hskey-authreq-abc ');
  expect(sent).toEqual(['hskey-authreq-abc']);
  expect(enrolment.view).toBe('registered');
  expect(enrolment.registered?.address).toBe('100.64.0.9');
});

it('vpn.md 6B: a refused auth ID keeps the register view with the hub error', async () => {
  const error = { code: 'VALIDATION', message: 'Not an auth ID.', fields: [] } as const;
  const { enrolment } = setup({ register: () => Promise.resolve({ ok: false, error }) });
  enrolment.openRegister();
  await enrolment.register('nope');
  expect(enrolment.view).toBe('register');
  expect(enrolment.error).toEqual(error);
});

it('VP-7: copying a step reports whether the clipboard took it', async () => {
  const { enrolment, copied } = setup();
  await enrolment.copy('sudo tailscale up');
  expect(copied).toEqual(['sudo tailscale up']);
  expect(enrolment.copied).toEqual({ text: 'sudo tailscale up', ok: true });
});
