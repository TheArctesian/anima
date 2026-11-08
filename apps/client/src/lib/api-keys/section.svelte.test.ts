// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import ApiKeysSection from './ApiKeysSection.svelte';
import { button, failure, fakeHub, FIXTURE_KEYS, NOW, ok, settled } from './api-keys.fixture';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

beforeEach(() => {
  installAnimations();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function open(answers: Parameters<typeof fakeHub>[0]): Promise<HTMLElement> {
  hub = fakeHub(answers);
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(ApiKeysSection, { target, props: { now: () => NOW } });
  flushSync();
  await settled();
  return target;
}

function row(target: HTMLElement, name: string): HTMLElement {
  const found = target.querySelector<HTMLElement>(`li[aria-label="${name}"]`);
  if (found === null) throw new Error(`no row ${name}`);
  return found;
}

/** The row's values by their labels. */
function values(element: HTMLElement): Record<string, string> {
  const pairs = [...element.querySelectorAll('dl > div')].map((pair) => [
    pair.querySelector('dt')?.textContent.trim() ?? '',
    pair.querySelector('dd')?.textContent.replace(/\s+/g, ' ').trim() ?? '',
  ]);
  return Object.fromEntries(pairs) as Record<string, string>;
}

it('INT-003: the section is a labelled region with the h2 "API keys" and lists each key with scope, prefix, expiry and last use', async () => {
  const target = await open({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }) });
  const section = target.querySelector('section');
  const heading = section?.querySelector('h2');
  expect(heading?.textContent).toBe('API keys');
  expect(section?.getAttribute('aria-labelledby')).toBe(heading?.id);
  expect([...target.querySelectorAll('li')].map((element) => element.getAttribute('aria-label'))).toEqual(['metrics-scrape', 'old-script']);
  const scrape = row(target, 'metrics-scrape');
  expect(values(scrape)).toMatchObject({ Scope: 'Read', Prefix: 'anima_Ab12Cd' });
  expect(scrape.querySelector('dd.mono')?.textContent).toBe('anima_Ab12Cd');
  expect(scrape.querySelector('time')?.getAttribute('datetime')).toBe('2027-06-01T00:00:00Z');
  expect(values(scrape).Expires).not.toContain('Expired');
  expect(values(scrape)['Last used']).not.toBe('Never');
});

it('UI-003: an expired key says "Expired" in text, and a key never used says "Never"', async () => {
  const target = await open({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }) });
  const old = values(row(target, 'old-script'));
  expect(old.Scope).toBe('Write');
  expect(old.Expires).toMatch(/Expired$/);
  expect(old['Last used']).toBe('Never');
});

it('INT-003: with no keys the section says so and still offers Create', async () => {
  const target = await open({ ApiKeys: ok({ apiKeys: [] }) });
  expect(target.querySelector('p')?.textContent).toBe("No API keys. Programs that call the hub's GraphQL API need one.");
  expect(button('Create API key')).toBeDefined();
});

it('INT-003: the section shows its loading line until the hub answers', async () => {
  let release: (result: unknown) => void = () => undefined;
  const pending = new Promise<never>((resolve) => {
    release = resolve as (result: unknown) => void;
  });
  const target = await open({ ApiKeys: pending });
  expect(target.querySelector('[role="status"]')?.textContent).toBe('Loading API keys…');
  release(ok({ apiKeys: FIXTURE_KEYS }));
  await settled();
  expect(target.querySelector('[role="status"]')).toBeNull();
  expect(target.querySelectorAll('li')).toHaveLength(2);
});

it('INT-003: a failed ApiKeys query shows only the section\'s fixed error, keeps Create, and Try again loads again', async () => {
  const target = await open({ ApiKeys: failure('INTERNAL', 'Unknown field "apiKeys"') });
  expect(target.querySelector('[role="alert"]')?.textContent).toBe('Anima could not load the API keys.');
  expect(target.textContent).not.toContain('Unknown field');
  expect(target.querySelector('h2')?.textContent).toBe('API keys');
  expect(button('Create API key').disabled).toBe(false);
  button('Try again').click();
  await settled();
  expect(hub.sent.map((request) => request.operation)).toEqual(['ApiKeys', 'ApiKeys']);
});

it('INT-003: Revoke asks first, naming the key, then sends RevokeApiKey with its id and the row leaves', async () => {
  const [scrape] = FIXTURE_KEYS;
  const target = await open({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }), RevokeApiKey: ok({ revokeApiKey: scrape?.id }) });
  row(target, 'metrics-scrape').querySelector('button')?.click();
  flushSync();
  const dialog = document.querySelector('[role="dialog"]');
  expect(dialog?.querySelector('h2')?.textContent).toBe('Revoke metrics-scrape?');
  expect(dialog?.textContent).toContain('Programs that use it stop working at once.');
  expect(hub.sent.map((request) => request.operation)).toEqual(['ApiKeys']);
  button('Revoke metrics-scrape').click();
  await settled();
  expect(hub.sent.at(-1)).toEqual({ operation: 'RevokeApiKey', variables: { id: scrape?.id } });
  expect([...target.querySelectorAll('li')].map((element) => element.getAttribute('aria-label'))).toEqual(['old-script']);
});

it('INT-003: Cancel in the revoke question sends nothing and keeps the row', async () => {
  const target = await open({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }) });
  row(target, 'old-script').querySelector('button')?.click();
  flushSync();
  button('Cancel').click();
  await settled();
  expect(hub.sent.map((request) => request.operation)).toEqual(['ApiKeys']);
  expect(target.querySelectorAll('li')).toHaveLength(2);
});

it('INT-003: a refused revoke keeps the row and the question says why', async () => {
  const target = await open({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }), RevokeApiKey: failure('unreachable', 'The hub did not answer. Check the VPN client.') });
  row(target, 'old-script').querySelector('button')?.click();
  flushSync();
  button('Revoke old-script').click();
  await settled();
  expect(document.querySelector('[role="dialog"] [role="alert"]')?.textContent).toBe('The hub did not answer. Check the VPN client.');
  expect(target.querySelectorAll('li')).toHaveLength(2);
});
