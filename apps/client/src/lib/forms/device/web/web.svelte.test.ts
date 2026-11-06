// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { HubResult } from '$lib/hub/error';
import { installAnimations } from '../../animations.fixture';
import type { WebInterface, WebInterfaceSuggestion } from '../../hub-pending';
import WebInterfaces from './WebInterfaces.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../motion/engine.fixture')).fakeRun,
}));

const OK: HubResult<unknown> = { ok: true, value: undefined };
const PVE: WebInterface = { id: 'w1', deviceId: 'd1', label: 'Proxmox VE', url: 'https://pve.lan:8006', openIn: 'ANIMA_TAB' };
const HA: WebInterface = { id: 'w2', deviceId: 'd1', label: 'Home Assistant', url: 'http://ha.lan:8123', openIn: 'SYSTEM_BROWSER' };
const SUGGESTION: WebInterfaceSuggestion = { checkId: 'c1', url: 'https://grafana.lan:3000', label: 'grafana.lan' };

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  installAnimations();
});

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function show(interfaces: WebInterface[], suggestions: WebInterfaceSuggestion[] = []) {
  const calls = {
    onCreate: vi.fn((): Promise<HubResult<unknown>> => Promise.resolve(OK)),
    onUpdate: vi.fn((): Promise<HubResult<unknown>> => Promise.resolve(OK)),
    onDelete: vi.fn((): Promise<HubResult<unknown>> => Promise.resolve(OK)),
    onAcceptSuggestion: vi.fn((): Promise<HubResult<unknown>> => Promise.resolve(OK)),
    onDismissSuggestion: vi.fn((): Promise<HubResult<unknown>> => Promise.resolve(OK)),
  };
  app = mount(WebInterfaces, { target: document.body, props: { interfaces, suggestions, calls } });
  flushSync();
  return calls;
}

async function settle(): Promise<void> {
  for (let turn = 0; turn < 4; turn += 1) await tick();
}

function button(label: string, within: ParentNode = document): HTMLButtonElement {
  const found = [...within.querySelectorAll('button')].find((candidate) => candidate.textContent.trim() === label);
  if (!found) throw new Error(`no button ${label}`);
  return found;
}

function row(label: string): HTMLElement {
  const found = document.querySelector<HTMLElement>(`li[aria-label="${label}"]`);
  if (!found) throw new Error(`no row ${label}`);
  return found;
}

function field(within: ParentNode, name: string): HTMLInputElement | HTMLSelectElement {
  const found = [...within.querySelectorAll('label')].find((candidate) => candidate.textContent === name);
  const control = found && document.getElementById(found.htmlFor);
  if (!(control instanceof HTMLInputElement || control instanceof HTMLSelectElement)) throw new Error(`no field ${name}`);
  return control;
}

function type(control: HTMLInputElement | HTMLSelectElement, text: string): void {
  control.value = text;
  control.dispatchEvent(new Event(control instanceof HTMLSelectElement ? 'change' : 'input', { bubbles: true }));
  flushSync();
}

it('WEB-001: each interface is a row with its label, URL and Open in, under a "Web interfaces" heading', () => {
  show([PVE, HA]);

  expect(document.querySelector('h3')?.textContent).toBe('Web interfaces');
  expect(field(row('Proxmox VE'), 'Label').value).toBe('Proxmox VE');
  expect(field(row('Proxmox VE'), 'URL').value).toBe('https://pve.lan:8006');
  expect(field(row('Proxmox VE'), 'Open in').value).toBe('ANIMA_TAB');
  expect(field(row('Home Assistant'), 'Open in').value).toBe('SYSTEM_BROWSER');
});

it('WEB-001: a device with none says so', () => {
  show([]);
  expect(document.body.textContent).toContain('no web interface');
});

it('D-6: an unchanged row does not save; a changed row saves at once with its own input', async () => {
  const calls = show([PVE, HA]);
  button('Save', row('Proxmox VE')).click();
  await settle();
  expect(calls.onUpdate).not.toHaveBeenCalled();

  type(field(row('Proxmox VE'), 'Open in'), 'SYSTEM_BROWSER');
  type(field(row('Proxmox VE'), 'Label'), '');
  button('Save', row('Proxmox VE')).click();
  await settle();

  expect(calls.onUpdate).toHaveBeenCalledExactlyOnceWith('w1', { label: null, url: 'https://pve.lan:8006', openIn: 'SYSTEM_BROWSER' });
});

it('WEB-001: Add opens an empty row; saving it calls onCreate with the input', async () => {
  const calls = show([]);
  button('Add a web interface').click();
  flushSync();
  expect(field(document, 'Open in').value).toBe('ANIMA_TAB');

  type(field(document, 'URL'), 'http://ha.lan:8123');
  button('Add web interface').click();
  await settle();

  expect(calls.onCreate).toHaveBeenCalledExactlyOnceWith({ label: null, url: 'http://ha.lan:8123', openIn: 'ANIMA_TAB' });
  expect(document.getElementById('web-new-url')).toBeNull();
});

it('WEB-001: a URL that is not http or https is flagged on the row and not sent', async () => {
  const calls = show([PVE]);
  type(field(row('Proxmox VE'), 'URL'), 'ftp://pve.lan');
  button('Save', row('Proxmox VE')).click();
  await settle();

  expect(calls.onUpdate).not.toHaveBeenCalled();
  const url = field(row('Proxmox VE'), 'URL');
  expect(url.getAttribute('aria-invalid')).toBe('true');
  expect(document.getElementById(url.getAttribute('aria-describedby') ?? '')?.textContent).toContain('http or https');
});

it('D-2: a hub field error shows on the row that was saved, not on another', async () => {
  const calls = show([PVE, HA]);
  calls.onUpdate.mockResolvedValueOnce({
    ok: false,
    error: { code: 'VALIDATION', message: 'Invalid', fields: [{ field: 'url', message: 'The hub refuses this URL.' }] },
  });
  type(field(row('Proxmox VE'), 'Label'), 'PVE');
  button('Save', row('Proxmox VE')).click();
  await settle();

  expect(row('Proxmox VE').textContent).toContain('The hub refuses this URL.');
  expect(row('Home Assistant').textContent).not.toContain('The hub refuses');
  expect(field(row('Proxmox VE'), 'URL').getAttribute('aria-invalid')).toBe('true');
});

it('DEV-014: Delete asks first and calls onDelete only after the confirmation', async () => {
  const calls = show([PVE]);
  button('Delete', row('Proxmox VE')).click();
  await settle();
  expect(calls.onDelete).not.toHaveBeenCalled();

  button('Delete Proxmox VE').click();
  await settle();

  expect(calls.onDelete).toHaveBeenCalledExactlyOnceWith('w1');
});

it('DEV-014: Cancel in the confirmation deletes nothing', async () => {
  const calls = show([PVE]);
  button('Delete', row('Proxmox VE')).click();
  await settle();
  button('Cancel').click();
  await settle();

  expect(calls.onDelete).not.toHaveBeenCalled();
});

it('WEB-002: a suggestion is offered with Add and Dismiss and is not in the list until accepted', async () => {
  const calls = show([PVE], [SUGGESTION]);
  expect(document.querySelectorAll('li[aria-label]')).toHaveLength(1);
  expect(document.body.textContent).toContain('https://grafana.lan:3000');

  button('Add').click();
  await settle();
  expect(calls.onAcceptSuggestion).toHaveBeenCalledExactlyOnceWith('c1');

  button('Dismiss').click();
  await settle();
  expect(calls.onDismissSuggestion).toHaveBeenCalledExactlyOnceWith('c1');
});

it('WEB-002: a failed accept shows the hub message', async () => {
  const calls = show([], [SUGGESTION]);
  calls.onAcceptSuggestion.mockResolvedValueOnce({ ok: false, error: { code: 'NOT_FOUND', message: 'That check is gone.' } });
  button('Add').click();
  await settle();

  expect(document.querySelector('[role="alert"]')?.textContent).toBe('That check is gone.');
});
