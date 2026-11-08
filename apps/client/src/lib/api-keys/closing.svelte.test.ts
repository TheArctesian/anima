// @vitest-environment jsdom
// How the shown key leaves: with the page, while the hub is still making it, and never while it is being made (F-8).
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import type { HubResult } from '$lib/hub/hub';
import ApiKeysSection from './ApiKeysSection.svelte';
import { button, fakeHub, field, FAKE_SECRET, FIXTURE_KEYS, MADE_KEY, NOW, ok, settled, typeInto } from './api-keys.fixture';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const CREATED = ok({ createApiKey: { key: MADE_KEY, secret: FAKE_SECRET } });
let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

beforeEach(() => {
  installAnimations();
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: vi.fn().mockResolvedValue(undefined) } });
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

/** Fades that never finish, so a test sees the document as it is during a fade. */
function holdAnimations(): void {
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value: () => ({ playState: 'running', currentTime: 0, cancel: () => undefined, pause: () => undefined, onfinish: null }),
  });
}

/** Whether a node keeps `text` in a property of its own, as a cache keyed by a symbol would. */
function nodeKeeps(element: Element, text: string): boolean {
  const own = element as unknown as Record<PropertyKey, unknown>;
  return [...Object.getOwnPropertyNames(element), ...Object.getOwnPropertySymbols(element)].some((name) => {
    try {
      return JSON.stringify(own[name])?.includes(text) === true;
    } catch {
      // A value JSON cannot write (a cycle) is a reference to an object, not a copy of the text.
      return false;
    }
  });
}

/** How many times the key appears in the markup, in field values, and in fields' own properties. */
function places(text: string): { markup: number; values: number; kept: number } {
  const fields = [...document.querySelectorAll('input, textarea')];
  const values = fields.filter((element) => (element as HTMLInputElement).value.includes(text));
  const kept = fields.filter((element) => nodeKeeps(element, text));
  return { markup: document.documentElement.outerHTML.split(text).length - 1, values: values.length, kept: kept.length };
}

async function openDialog(createAnswer: HubResult<unknown> | Promise<HubResult<unknown>>, now: () => Date = () => NOW): Promise<void> {
  hub = fakeHub({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }), CreateApiKey: createAnswer });
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(ApiKeysSection, { target, props: { now } });
  flushSync();
  await settled();
  button('Create API key').click();
  flushSync();
  typeInto(field('[role="dialog"] input[type="text"]'), 'backup-job');
}

function pending(): { answer: Promise<HubResult<unknown>>; release: (result: HubResult<unknown>) => void } {
  let release: (result: HubResult<unknown>) => void = () => undefined;
  const answer = new Promise<HubResult<unknown>>((resolve) => {
    release = resolve;
  });
  return { answer, release };
}

it('INT-003: while shown, the key is in exactly one place: the field value, never the markup', async () => {
  await openDialog(CREATED);
  button('Create key').click();
  await settled();
  expect(places(FAKE_SECRET)).toEqual({ markup: 0, values: 1, kept: 0 });
});

it('F-8: leaving Settings while the key shows empties the field at once, though the dialog is still fading', async () => {
  await openDialog(CREATED);
  button('Create key').click();
  await settled();
  holdAnimations();
  if (app !== undefined) void unmount(app, { outro: true });
  app = undefined;
  flushSync();
  expect(places(FAKE_SECRET)).toEqual({ markup: 0, values: 0, kept: 0 });
  await settled();
  expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  expect(places(FAKE_SECRET)).toEqual({ markup: 0, values: 0, kept: 0 });
});

/** Opens the dialog and presses Create; the hub has not answered yet. */
async function making(): Promise<(result: HubResult<unknown>) => void> {
  const { answer, release } = pending();
  await openDialog(answer);
  button('Create key').click();
  flushSync();
  return release;
}

it('F-8: Escape is refused while the key is being made, and the key shows when it arrives', async () => {
  const release = await making();
  document.querySelector('[role="dialog"]')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  await settled();
  expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  release(CREATED);
  await settled();
  expect(field('[role="dialog"] input[readonly]').value).toBe(FAKE_SECRET);
});

it('F-8: a click beside the dialog is refused while the key is being made', async () => {
  const release = await making();
  document.querySelector('.frame')?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
  await settled();
  expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  release(CREATED);
  await settled();
  expect(field('[role="dialog"] input[readonly]').value).toBe(FAKE_SECRET);
});

it('F-8: Cancel is disabled while the key is being made', async () => {
  const release = await making();
  // The form's fieldset disables it, so the button's own `disabled` stays false; `:disabled` is what the browser honours.
  expect(button('Cancel').matches(':disabled')).toBe(true);
  button('Cancel').click();
  await settled();
  expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  release(CREATED);
  await settled();
  expect(field('[role="dialog"] input[readonly]').value).toBe(FAKE_SECRET);
});

it('F-8: a page left while the key is being made never puts the key in the document when the hub answers', async () => {
  const { answer, release } = pending();
  await openDialog(answer);
  button('Create key').click();
  flushSync();
  holdAnimations();
  if (app !== undefined) void unmount(app, { outro: true });
  app = undefined;
  flushSync();
  release(CREATED);
  await settled();
  expect(places(FAKE_SECRET)).toEqual({ markup: 0, values: 0, kept: 0 });
});

it('D-23: the expiry counts from when Create is pressed, not from when the dialog opened', async () => {
  let clock = NOW;
  await openDialog(CREATED, () => clock);
  clock = new Date(NOW.getTime() + 86_400_000);
  button('Create key').click();
  await settled();
  expect(hub.sent.at(-1)).toEqual({
    operation: 'CreateApiKey',
    variables: { input: { name: 'backup-job', scope: 'READ', expiresAt: '2027-02-06T12:00:00.000Z' } },
  });
});
