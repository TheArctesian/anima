// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import type { LoadingClock } from '$lib/motion/loading/loading';
import { threshold } from '$lib/motion/tokens';
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

async function open(answers: Parameters<typeof fakeHub>[0], clock?: LoadingClock): Promise<HTMLElement> {
  hub = fakeHub(answers);
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(ApiKeysSection, { target, props: { now: () => NOW, clock } });
  flushSync();
  await settled();
  return target;
}

/** The listed keys' names, in order. */
function names(target: HTMLElement): (string | null)[] {
  return [...target.querySelectorAll('.list > li')].map((element) => element.getAttribute('aria-label'));
}

function pending(): { answer: Promise<never>; release: (result: unknown) => void } {
  let release: (result: unknown) => void = () => undefined;
  const answer = new Promise<never>((resolve) => {
    release = resolve as (result: unknown) => void;
  });
  return { answer, release };
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

it('INT-003: the section lists each key with scope, prefix, expiry and last use', async () => {
  const target = await open({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }) });
  expect(names(target)).toEqual(['metrics-scrape', 'old-script']);
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
  expect(target.querySelector('p.quiet')?.textContent).toBe("No API keys. Programs that call the hub's GraphQL API need one.");
  expect(button('Create API key')).toBeDefined();
});

it('MOT-007: after threshold-loading a skeleton key row shows, hidden from assistive technology, and the answer replaces it', async () => {
  const time = manualClock();
  const { answer, release } = pending();
  const target = await open({ ApiKeys: answer }, time.clock);
  const stack = target.querySelector('.stack');
  expect(stack?.getAttribute('aria-busy')).toBe('true');
  time.advance(threshold.loading - 1);
  flushSync();
  expect(target.querySelector('.skeleton')).toBeNull();
  time.advance(1);
  flushSync();
  const skeleton = target.querySelector('.skeleton');
  expect([skeleton?.getAttribute('aria-hidden'), skeleton?.hasAttribute('inert'), skeleton?.parentElement]).toEqual(['true', true, stack]);
  expect(skeleton?.querySelectorAll(':scope > li.placeholder > dl > div')).toHaveLength(4);
  release(ok({ apiKeys: FIXTURE_KEYS }));
  await settled();
  time.advance(threshold.loadingMin);
  await settled();
  expect(target.querySelector('.skeleton')).toBeNull();
  expect(stack?.getAttribute('aria-busy')).toBe('false');
  expect(names(target)).toEqual(['metrics-scrape', 'old-script']);
});

it('MOT-007: a hidden key row keeps one key\'s space in the cell where the list, the empty line and the failure land', async () => {
  const answers = [ok({ apiKeys: FIXTURE_KEYS.slice(0, 1) }), ok({ apiKeys: [] }), failure('INTERNAL')];
  const landed = ['.list', 'p.quiet', '.failed'];
  for (const [index, answer] of answers.entries()) {
    const target = await open({ ApiKeys: answer });
    const sizer = target.querySelector('.stack > .sizer');
    expect([sizer?.getAttribute('aria-hidden'), sizer?.hasAttribute('inert')]).toEqual(['true', true]);
    expect(sizer?.querySelectorAll(':scope > li.placeholder > dl > div')).toHaveLength(4);
    expect(target.querySelector(`.stack > ${landed[index] ?? ''}`)).not.toBeNull();
    if (app !== undefined) await unmount(app);
    app = undefined;
  }
});

it('INT-003: a failed ApiKeys query shows only the section\'s fixed error, keeps Create, and Try again loads again', async () => {
  const target = await open({ ApiKeys: failure('INTERNAL', 'Unknown field "apiKeys"') });
  expect(target.querySelector('[role="alert"]')?.textContent).toBe('Anima could not load the API keys.');
  expect(target.textContent).not.toContain('Unknown field');
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
  expect(names(target)).toEqual(['old-script']);
});

it('INT-003: Cancel in the revoke question sends nothing and keeps the row', async () => {
  const target = await open({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }) });
  row(target, 'old-script').querySelector('button')?.click();
  flushSync();
  button('Cancel').click();
  await settled();
  expect(hub.sent.map((request) => request.operation)).toEqual(['ApiKeys']);
  expect(names(target)).toHaveLength(2);
});

it('INT-003: a refused revoke keeps the row and the question says why', async () => {
  const target = await open({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }), RevokeApiKey: failure('unreachable', 'The hub did not answer. Check the VPN client.') });
  row(target, 'old-script').querySelector('button')?.click();
  flushSync();
  button('Revoke old-script').click();
  await settled();
  expect(document.querySelector('[role="dialog"] [role="alert"]')?.textContent).toBe('The hub did not answer. Check the VPN client.');
  expect(names(target)).toHaveLength(2);
});

/** Opens the revoke question for `name` and confirms it. */
async function revokeRow(target: HTMLElement, name: string): Promise<void> {
  row(target, name).querySelector('button')?.click();
  flushSync();
  button(`Revoke ${name}`).click();
  await settled();
}

it('NAV-005: after a revoke, focus goes to the row that took its place', async () => {
  const target = await open({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }), RevokeApiKey: ok({ revokeApiKey: FIXTURE_KEYS[0]?.id }) });
  await revokeRow(target, 'metrics-scrape');
  expect(document.activeElement).toBe(row(target, 'old-script').querySelector('button'));
});

it('NAV-005: after revoking the last row, focus goes to the row before it', async () => {
  const target = await open({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS }), RevokeApiKey: ok({ revokeApiKey: FIXTURE_KEYS[1]?.id }) });
  await revokeRow(target, 'old-script');
  expect(document.activeElement).toBe(row(target, 'metrics-scrape').querySelector('button'));
});

it('NAV-005: after revoking the only key, focus goes to "Create API key"', async () => {
  const target = await open({ ApiKeys: ok({ apiKeys: FIXTURE_KEYS.slice(0, 1) }), RevokeApiKey: ok({ revokeApiKey: FIXTURE_KEYS[0]?.id }) });
  await revokeRow(target, 'metrics-scrape');
  expect(names(target)).toEqual([]);
  expect(document.activeElement).toBe(button('Create API key'));
});

it('MOT-007, NAV-005: Try again stays mounted and keeps focus while it retries and when it fails again, then focus goes to Create', async () => {
  const answers: Parameters<typeof fakeHub>[0] = { ApiKeys: failure('INTERNAL') };
  await open(answers);
  const again = button('Try again');
  again.focus();
  again.click();
  flushSync();
  expect([document.activeElement === again, again.getAttribute('aria-disabled'), again.getAttribute('aria-busy')]).toEqual([true, 'true', 'true']);
  again.click();
  await settled();
  expect(hub.sent.filter((each) => each.operation === 'ApiKeys')).toHaveLength(2);
  await settled();
  expect([document.activeElement === again, again.isConnected, again.getAttribute('aria-disabled')]).toEqual([true, true, 'false']);
  answers.ApiKeys = ok({ apiKeys: FIXTURE_KEYS });
  again.click();
  await settled();
  await settled();
  expect(document.activeElement).toBe(button('Create API key'));
});
