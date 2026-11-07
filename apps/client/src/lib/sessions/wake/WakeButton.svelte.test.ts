// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import type { WakeApi, WakeSent } from './api';
import WakeButton from './WakeButton.svelte';

const mounted: object[] = [];

afterEach(async () => {
  for (const component of mounted.splice(0)) await unmount(component);
  document.body.innerHTML = '';
});

/** A wake call that stays pending until `answer` is called. */
function pendingApi() {
  const calls: string[] = [];
  let answer: (result: HubResult<WakeSent>) => void = () => undefined;
  const api: WakeApi = {
    wakeDevice: (deviceId) => {
      calls.push(deviceId);
      return new Promise((resolve) => {
        answer = resolve;
      });
    },
  };
  return { api, calls, answer: (result: HubResult<WakeSent>) => answer(result) };
}

function show(api: WakeApi, disabled = false) {
  mounted.push(mount(WakeButton, { target: document.body, props: { deviceId: 'device-1', api, disabled } }));
  flushSync();
  return {
    button: document.querySelector('button') as HTMLButtonElement,
    status: () => document.querySelector('[role="status"]')?.textContent.trim() ?? '',
  };
}

const settle = async () => {
  await Promise.resolve();
  await Promise.resolve();
  flushSync();
};

it('RA-031: a press asks the hub once for this device, and a second press while it waits is ignored', async () => {
  const { api, calls, answer } = pendingApi();
  const { button } = show(api);
  button.click();
  flushSync();
  expect(button.disabled).toBe(true);
  button.click();
  expect(calls).toEqual(['device-1']);
  answer({ ok: true, value: { sentAt: '2026-11-06T10:00:00Z', relayName: 'Pi' } });
  await settle();
  expect(button.disabled).toBe(false);
});

it('RA-031: the status line names the relay after a success', async () => {
  const { api, answer } = pendingApi();
  const { button, status } = show(api);
  button.click();
  answer({ ok: true, value: { sentAt: '2026-11-06T10:00:00Z', relayName: 'Pi at home' } });
  await settle();
  expect(status()).toBe('Wake packet sent through Pi at home.');
});

it('RA-031: the status line shows the fixed text of a refusal', async () => {
  const { api, answer } = pendingApi();
  const { button, status } = show(api);
  button.click();
  answer({ ok: false, error: { code: 'CONFLICT', message: 'relay 10.0.0.9' } });
  await settle();
  expect(status()).toContain('MAC address');
  expect(status()).not.toContain('10.0.0.9');
});

it('RA-031: a disabled button sends nothing', () => {
  const { api, calls } = pendingApi();
  const { button } = show(api, true);
  button.click();
  expect(calls).toEqual([]);
});
