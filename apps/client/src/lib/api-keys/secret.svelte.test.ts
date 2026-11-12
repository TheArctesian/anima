// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import ApiKeysSection from './ApiKeysSection.svelte';
import { button, failure, fakeHub, field, keyField, FAKE_SECRET, FIXTURE_KEYS, MADE_KEY, NOW, ok, radio, settled, typeInto } from './api-keys.fixture';
import { DATE_MISSING, DATE_TOO_FAR, NAME_MISSING, NAME_TOO_LONG } from './new-key';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const CONSOLE = ['log', 'info', 'warn', 'error', 'debug'] as const;
const consoleSpies = CONSOLE.map((name) => vi.spyOn(console, name).mockImplementation(() => undefined));
const writeText = vi.fn<(text: string) => Promise<void>>();
let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

beforeEach(() => {
  installAnimations();
  for (const spy of consoleSpies) spy.mockClear();
  writeText.mockReset().mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } });
  localStorage.clear();
  sessionStorage.clear();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const CREATED = ok({ createApiKey: { key: MADE_KEY, secret: FAKE_SECRET } });

async function openDialog(answers: Parameters<typeof fakeHub>[0] = {}): Promise<HTMLElement> {
  hub = fakeHub({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }), ...answers });
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(ApiKeysSection, { target, props: { now: () => NOW } });
  flushSync();
  await settled();
  button('Create API key').click();
  flushSync();
  return target;
}

function nameField(): HTMLInputElement {
  return field('[role="dialog"] input[type="text"]');
}

function problems(): string[] {
  return [...document.querySelectorAll('[role="dialog"] .problem')].map((element) => element.textContent).filter((text) => text !== '');
}

async function create(): Promise<void> {
  button('Create key').click();
  await settled();
}

/** Every place in the document the key could be: markup, text, attributes and field values. */
function documentHolds(text: string): boolean {
  const values = [...document.querySelectorAll('input, textarea')].map((element) => (element as HTMLInputElement).value);
  return document.documentElement.outerHTML.includes(text) || values.some((value) => value.includes(text)) || document.title.includes(text);
}

it('INT-003: an empty name is refused before anything is sent', async () => {
  await openDialog();
  await create();
  expect(problems()).toEqual([NAME_MISSING]);
  expect(nameField().getAttribute('aria-invalid')).toBe('true');
  expect(hub.sent.map((request) => request.operation)).toEqual(['ApiKeys']);
});

it('INT-003: a name over 64 characters is refused before anything is sent', async () => {
  await openDialog();
  typeInto(nameField(), 'k'.repeat(65));
  await create();
  expect(problems()).toEqual([NAME_TOO_LONG]);
  expect(hub.sent.map((request) => request.operation)).toEqual(['ApiKeys']);
});

it('D-23: "On a date" without a date is refused before anything is sent', async () => {
  await openDialog();
  typeInto(nameField(), 'backup-job');
  radio('On a date').click();
  flushSync();
  await create();
  expect(problems()).toEqual([DATE_MISSING]);
  expect(hub.sent.map((request) => request.operation)).toEqual(['ApiKeys']);
});

it('D-23: a date beyond one year is refused before anything is sent', async () => {
  await openDialog();
  typeInto(nameField(), 'backup-job');
  radio('On a date').click();
  flushSync();
  const date = field('[role="dialog"] input[type="date"]');
  expect(date.disabled).toBe(false);
  typeInto(date, '2027-11-09');
  await create();
  expect(problems()).toEqual([DATE_TOO_FAR]);
  expect(hub.sent.map((request) => request.operation)).toEqual(['ApiKeys']);
});

it('INT-003: Create sends CreateApiKey with the name, the scope and the default 90-day expiry', async () => {
  await openDialog({ CreateApiKey: CREATED });
  typeInto(nameField(), 'backup-job');
  radio('Write').click();
  flushSync();
  await create();
  expect(hub.sent.at(-1)).toEqual({
    operation: 'CreateApiKey',
    variables: { input: { name: 'backup-job', scope: 'WRITE', expiresAt: '2027-02-05T12:00:00.000Z' } },
  });
});

it('INT-003: a refused create shows a fixed sentence, not the hub text, and keeps the form', async () => {
  await openDialog({ CreateApiKey: failure('CONFLICT', 'duplicate key value "backup-job"') });
  typeInto(nameField(), 'backup-job');
  await create();
  expect(document.querySelector('[role="dialog"] [role="alert"]')?.textContent).toBe('This name is taken.');
  expect(nameField().value).toBe('backup-job');
});

it('MOT-007: the failure line is mounted before a refusal, so the buttons stay in place', async () => {
  await openDialog({ CreateApiKey: failure('CONFLICT', 'duplicate key value "backup-job"') });
  const line = document.querySelector('[role="dialog"] form [role="alert"]');
  expect(line?.textContent).toBe('');
  typeInto(nameField(), 'backup-job');
  await create();
  expect(document.querySelector('[role="dialog"] form [role="alert"]')).toBe(line);
  expect(line?.textContent).toBe('This name is taken.');
});

it('INT-003: after Create the key shows once with the once-only sentence, and the list gains the key without it', async () => {
  const target = await openDialog({ CreateApiKey: CREATED });
  typeInto(nameField(), 'backup-job');
  await create();
  const dialog = document.querySelector('[role="dialog"]');
  const shown = keyField();
  expect(shown.value).toBe(FAKE_SECRET);
  expect(document.activeElement).toBe(shown);
  expect(dialog?.textContent).toContain('Anima shows this key only once.');
  expect(dialog?.getAttribute('aria-describedby')).toBe(dialog?.querySelector('p')?.id);
  expect(target.querySelector('li[aria-label="backup-job"]')).not.toBeNull();
  expect(target.textContent).not.toContain(FAKE_SECRET);
  expect([...document.querySelectorAll('[title]')].some((element) => element.getAttribute('title')?.includes(FAKE_SECRET))).toBe(false);
  expect([...document.querySelectorAll('[role="status"], [aria-live]')].some((element) => element.textContent.includes(FAKE_SECRET))).toBe(false);
});

it('INT-003: closing the dialog takes the key out of the document at once, before the fade ends', async () => {
  await openDialog({ CreateApiKey: CREATED });
  typeInto(nameField(), 'backup-job');
  await create();
  button('Done').click();
  expect(documentHolds(FAKE_SECRET)).toBe(false);
  await settled();
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  expect(documentHolds(FAKE_SECRET)).toBe(false);
});

it('INT-003: Escape closes the shown key too, and a new dialog starts empty without it', async () => {
  await openDialog({ CreateApiKey: CREATED });
  typeInto(nameField(), 'backup-job');
  await create();
  document.querySelector('[role="dialog"]')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  flushSync();
  expect(documentHolds(FAKE_SECRET)).toBe(false);
  await settled();
  button('Create API key').click();
  flushSync();
  expect(nameField().value).toBe('');
  expect(documentHolds(FAKE_SECRET)).toBe(false);
});

it('LOG-007: the key reaches no console method, no storage and no URL', async () => {
  const setItem = vi.spyOn(Storage.prototype, 'setItem');
  const href = location.href;
  await openDialog({ CreateApiKey: CREATED });
  typeInto(nameField(), 'backup-job');
  await create();
  button('Copy').click();
  await settled();
  button('Done').click();
  await settled();
  for (const spy of consoleSpies) expect(JSON.stringify(spy.mock.calls)).not.toContain(FAKE_SECRET);
  for (const spy of consoleSpies) expect(spy).not.toHaveBeenCalled();
  expect(setItem).not.toHaveBeenCalled();
  expect([localStorage.length, sessionStorage.length]).toEqual([0, 0]);
  expect(location.href).toBe(href);
  setItem.mockRestore();
});

it('INT-003: Copy puts the key on the clipboard and says so, without the key', async () => {
  await openDialog({ CreateApiKey: CREATED });
  typeInto(nameField(), 'backup-job');
  await create();
  button('Copy').click();
  await settled();
  expect(writeText).toHaveBeenCalledExactlyOnceWith(FAKE_SECRET);
  expect(document.querySelector('[role="dialog"] [role="status"]')?.textContent.trim()).toBe('Copied.');
});

it('INT-003: a refused copy says so and leaves the key selectable in its field', async () => {
  writeText.mockRejectedValue(new Error('denied'));
  await openDialog({ CreateApiKey: CREATED });
  typeInto(nameField(), 'backup-job');
  await create();
  button('Copy').click();
  await settled();
  const outcome = document.querySelector('[role="dialog"] [role="status"]')?.textContent.trim() ?? '';
  expect(outcome).toBe('Anima could not copy the key. Select it in the field and copy it yourself.');
  expect(outcome).not.toContain(FAKE_SECRET);
  const shown = keyField();
  expect([shown.value, shown.disabled]).toEqual([FAKE_SECRET, false]);
  for (const spy of consoleSpies) expect(spy).not.toHaveBeenCalled();
});

it('INT-003: a click beside the dialog does not close the shown key, so it cannot be lost by a slip', async () => {
  await openDialog({ CreateApiKey: CREATED });
  typeInto(nameField(), 'backup-job');
  await create();
  document.querySelector('.frame')?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
  await settled();
  expect(keyField().value).toBe(FAKE_SECRET);
});

it('MOT-007 / MV-002: the secret view keeps the form\'s height, so the surface does not drop in one frame', async () => {
  await openDialog({ CreateApiKey: CREATED });
  typeInto(nameField(), 'backup-job');
  const views = document.querySelector<HTMLElement>('[role="dialog"] .views');
  // jsdom has no layout: the form is given the height a browser measured (566 px surface, the form 450 px).
  vi.spyOn(views as HTMLElement, 'getBoundingClientRect').mockReturnValue(new DOMRect(0, 0, 400, 450));
  expect(views?.style.minBlockSize).toBe('');
  await create();
  expect(document.querySelector('[role="dialog"] form')).toBeNull();
  expect(views?.style.minBlockSize).toBe('450px');
});
