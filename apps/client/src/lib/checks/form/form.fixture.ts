import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeAll } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import CheckForm from './CheckForm.svelte';
import type { SiblingCheck } from './model/group';
import type { CheckFields } from './model/hub-types';
import type { CheckFormValues } from './model/values';

export const OK: HubResult<unknown> = { ok: true, value: {} };

let app: ReturnType<typeof mount> | undefined;

/** Call once at the top of a component test file: stubs Web Animations and unmounts after each test. */
export function installFormHarness(): void {
  // jsdom has no Web Animations; Svelte's transitions need an animation that never finishes on its own.
  beforeAll(() => {
    Element.prototype.animate = () =>
      ({ onfinish: null, cancel: () => undefined, finish: () => undefined, currentTime: 0 }) as unknown as Animation;
  });
  afterEach(() => {
    if (app !== undefined) void unmount(app);
    app = undefined;
    document.body.innerHTML = '';
  });
}

interface Options {
  check?: CheckFields | null;
  siblings?: SiblingCheck[];
  onSave?: (values: CheckFormValues) => Promise<HubResult<unknown>>;
  onDone?: () => void;
}

export function show({ check = null, siblings = [], onSave = () => Promise.resolve(OK), onDone }: Options = {}): HTMLFormElement {
  app = mount(CheckForm, { target: document.body, props: { check, siblings, onSave, onDone, onCancel: () => undefined } });
  flushSync();
  return document.querySelector('form') as HTMLFormElement;
}

/** The input, select or textarea whose label reads `label`. */
export function input(label: string): HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement {
  const found = [...document.querySelectorAll('label')].find((candidate) => candidate.textContent.trim() === label);
  const target = found?.control ?? null;
  if (target === null) throw new Error(`no input labelled ${label}`);
  return target as HTMLInputElement;
}

export function hasInput(label: string): boolean {
  return [...document.querySelectorAll('label')].some((candidate) => candidate.textContent.trim() === label);
}

export function type(label: string, value: string): void {
  const field = input(label);
  field.value = value;
  field.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

export function choose(label: string, value: string): void {
  const field = input(label);
  field.value = value;
  field.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
}

export function button(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find((candidate) => candidate.textContent.includes(label));
  if (found === undefined) throw new Error(`no button ${label}`);
  return found;
}

export async function settle(): Promise<void> {
  await tick();
  await Promise.resolve();
  flushSync();
}

export async function submit(form: HTMLFormElement): Promise<void> {
  form.dispatchEvent(new SubmitEvent('submit', { bubbles: true, cancelable: true }));
  await settle();
}

/** The hint text shown for `label`'s input through `aria-describedby`; empty when there is none. */
export function messageOfHint(label: string): string {
  const described = input(label).getAttribute('aria-describedby') ?? '';
  return described
    .split(' ')
    .map((id) => document.getElementById(id))
    .filter((element) => element !== null && !element.classList.contains('error'))
    .map((element) => element?.textContent ?? '')
    .join(' ');
}

/** The message shown for `field`'s input through `aria-describedby`. */
export function messageOf(label: string): string | undefined {
  const described = input(label).getAttribute('aria-describedby') ?? '';
  return described
    .split(' ')
    .map((id) => document.getElementById(id))
    .find((element) => element?.classList.contains('error'))?.textContent;
}
