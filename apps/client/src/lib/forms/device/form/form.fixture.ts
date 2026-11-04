import { flushSync, mount, tick, unmount } from 'svelte';
import { vi } from 'vitest';
import type { HubResult } from '$lib/hub/error';
import type { ConnectionTestInput, ConnectionTestResult, DeviceFields, HostKey } from '../hub-types';
import type { DeviceFormValues } from '../values';
import DeviceForm from './DeviceForm.svelte';

export const OK: HubResult<unknown> = { ok: true, value: undefined };
export const SAVED: DeviceFields = {
  name: 'nas',
  host: 'nas.home.arpa',
  kind: 'server',
  os: 'LINUX',
  tags: ['lab'],
  ssh: { enabled: true, port: 22, username: 'root', auth: 'PASSWORD' },
};
export const HOST_KEY: HostKey = { algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:abc', firstSeenAt: '2026-11-04T09:00:00Z' };
export const KEY = '-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n-----END OPENSSH PRIVATE KEY-----\n';

let app: ReturnType<typeof mount> | undefined;

/** Unmounts the form and empties the page; call it after each test. */
export function leave(): void {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
}

export interface Calls {
  onSave: ReturnType<typeof vi.fn<(values: DeviceFormValues, siteId: string | null) => Promise<HubResult<unknown>>>>;
  onTest: ReturnType<typeof vi.fn<(input: ConnectionTestInput) => Promise<HubResult<ConnectionTestResult>>>>;
  onAcceptHostKey: ReturnType<typeof vi.fn<(fingerprint: string) => Promise<HubResult<unknown>>>>;
}

export function show(device: DeviceFields | null, options: Record<string, unknown> = {}, tested?: HubResult<ConnectionTestResult>): Calls {
  const calls: Calls = {
    onSave: vi.fn(() => Promise.resolve(OK)),
    onTest: vi.fn(() => Promise.resolve(tested ?? { ok: true as const, value: { reachable: true, authenticated: true, hostKeyChanged: false } })),
    onAcceptHostKey: vi.fn(() => Promise.resolve(OK)),
  };
  app = mount(DeviceForm, {
    target: document.body,
    props: {
      device,
      deviceId: device === null ? null : 'd1',
      ...calls,
      onClearCredential: () => Promise.resolve(OK),
      onCancel: () => undefined,
      ...options,
    },
  });
  flushSync();
  return calls;
}

export function input(id: string): HTMLInputElement {
  const found = document.getElementById(`device-${id}`);
  if (!(found instanceof HTMLInputElement || found instanceof HTMLTextAreaElement)) throw new Error(`no input ${id}`);
  return found as HTMLInputElement;
}

export function type(id: string, text: string): void {
  const field = input(id);
  field.value = text;
  field.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

export function button(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find((candidate) => candidate.textContent.trim() === label);
  if (!found) throw new Error(`no button ${label}`);
  return found;
}

export async function settle(): Promise<void> {
  for (let turn = 0; turn < 4; turn += 1) await tick();
}

export async function save(): Promise<void> {
  button('Save').click();
  await settle();
}

export function choose(value: 'PASSWORD' | 'PRIVATE_KEY'): void {
  const radio = document.querySelector<HTMLInputElement>(`input[type="radio"][value="${value}"]`);
  radio?.click();
  flushSync();
}
