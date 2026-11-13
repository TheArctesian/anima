// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import BuildsView from '../BuildsView.svelte';
import { fakeHub, job, ok, release, settled, type Answers } from './builds-view.fixture';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;

/** Three attempts at 0.1.1, as an operator makes them after fixing a failed build, and one older release. */
const ATTEMPTS = [
  release('rel-0', '0.1.0', '2026-11-10T10:00:00Z', [job({ id: 'job-0', releaseId: 'rel-0', state: 'PUBLISHED' })]),
  release('rel-1', '0.1.1', '2026-11-12T21:28:00Z', [
    job({ id: 'job-1', releaseId: 'rel-1', target: 'DARWIN_UNIVERSAL', state: 'FAILED', failureReason: 'The gates failed. The job log says why.' }),
  ]),
  release('rel-2', '0.1.1', '2026-11-12T21:55:00Z', [
    job({ id: 'job-2', releaseId: 'rel-2', target: 'DARWIN_UNIVERSAL', state: 'FAILED', failureReason: 'The bundle failed. The job log says why.' }),
  ]),
  release('rel-3', '0.1.1', '2026-11-12T22:20:00Z', [
    job({ id: 'job-3', releaseId: 'rel-3', target: 'DARWIN_UNIVERSAL', state: 'BUILDING', builderName: 'mac' }),
    job({ id: 'job-4', releaseId: 'rel-3', state: 'QUEUED' }),
  ]),
];

beforeEach(() => {
  installAnimations();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function open(releases = ATTEMPTS, answers: Answers = {}): Promise<HTMLElement> {
  const hub = fakeHub({
    Releases: ok({ releases: { edges: releases.map((node) => ({ node })) } }),
    NextReleaseVersion: ok({ nextReleaseVersion: '0.1.2' }),
    Builders: ok({ builders: [] }),
    BuildJobLog: ok({ buildJobLog: [] }),
    ...answers,
  });
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(BuildsView, { target });
  flushSync();
  await settled();
  return target;
}

const cards = (target: HTMLElement) => [...target.querySelectorAll('article')];
const shown = (element: Element) => element.closest('[hidden]') === null;
/** An element's words, without the icon glyphs (private use characters) a screen reader is not given. */
const words = (element: Element) => element.textContent.replaceAll(/[\u{E000}-\u{F8FF}\u{F0000}-\u{FFFFD}]/gu, '').replaceAll(/\s+/g, ' ').trim();
const disclosure = (target: HTMLElement) =>
  [...target.querySelectorAll('button')].find((button) => words(button) === '2 earlier attempts at 0.1.1');

function card(target: HTMLElement, name: string): HTMLElement {
  const found = cards(target).find((element) => element.getAttribute('aria-label') === name);
  if (found === undefined) throw new Error(`no release ${name}`);
  return found;
}

it('BLD-080: the newest release comes first and is the prominent one, with its state and published count', async () => {
  const target = await open();
  const first = cards(target)[0];
  expect(first?.getAttribute('aria-label')).toBe('Release 0.1.1');
  expect(first?.classList.contains('prominent')).toBe(true);
  expect(first?.querySelector('header')?.textContent).toContain('Building');
  expect(first?.querySelector('header')?.textContent).toContain('0 of 2 published');
  expect(cards(target).filter((element) => element.classList.contains('prominent'))).toHaveLength(1);
});

it('BLD-080: earlier attempts at a version are folded under the newest, and open on request', async () => {
  const target = await open();
  const toggle = disclosure(target);
  expect(toggle?.getAttribute('aria-expanded')).toBe('false');
  expect(shown(card(target, 'Release 0.1.1, attempt 2'))).toBe(false);
  toggle?.click();
  await settled();
  expect(toggle?.getAttribute('aria-expanded')).toBe('true');
  const earlier = [card(target, 'Release 0.1.1, attempt 2'), card(target, 'Release 0.1.1, attempt 1')];
  expect(earlier.map((element) => element.textContent.includes('The bundle failed'))).toEqual([true, false]);
  expect(earlier.every(shown)).toBe(true);
});

it('BLD-080: each attempt names its jobs apart, so every log button says which attempt it opens', async () => {
  const target = await open();
  const names = [...target.querySelectorAll('button[aria-pressed]')].map((button) => button.getAttribute('aria-label'));
  expect(names).toEqual([
    'Show log of darwin-universal job of 0.1.1',
    'Show log of linux-x86_64 job of 0.1.1',
    'Show log of darwin-universal job of 0.1.1, attempt 2',
    'Show log of darwin-universal job of 0.1.1, attempt 1',
    'Show log of linux-x86_64 job of 0.1.0',
  ]);
});

it('BLD-080: an earlier attempt that is still running is not folded away', async () => {
  const running = [
    release('rel-1', '0.1.1', '2026-11-12T21:28:00Z', [job({ id: 'job-1', releaseId: 'rel-1', state: 'BUILDING' })]),
    release('rel-2', '0.1.1', '2026-11-12T21:55:00Z', [job({ id: 'job-2', releaseId: 'rel-2', state: 'QUEUED' })]),
  ];
  const target = await open(running);
  expect(shown(card(target, 'Release 0.1.1, attempt 1'))).toBe(true);
});

it('BLD-080: a job under way or failed shows its stages, the failed one the step that failed', async () => {
  const target = await open();
  disclosure(target)?.click();
  await settled();
  const tracks = [...target.querySelectorAll('ol[aria-label="Stages"]')].map(words);
  expect(tracks[0]).toBe('Claim: done Build: in progress checkout, gates, bundle, notarize Upload: not started Publish: not started');
  expect(tracks).toContain('Claim: done Build: failed the bundle failed Upload: not run Publish: not run');
  // The published 0.1.0 job has no track: its state says every stage is done.
  expect(tracks).toHaveLength(4);
});

it('BLD-081: the log marks failure lines and keeps line numbers out of its text', async () => {
  const target = await open(ATTEMPTS, {
    BuildJobLog: ok({
      buildJobLog: [
        { sequence: 1, text: '@anima/client test: (pass) one' },
        { sequence: 2, text: '@anima/client test: (fail) two' },
      ],
    }),
  });
  [...target.querySelectorAll('button')].find((button) => button.getAttribute('aria-label') === 'Show log of darwin-universal job of 0.1.1')?.click();
  await settled();
  const log = target.querySelector('[role="log"][aria-live="polite"]');
  expect(log?.querySelector('pre')?.textContent).toBe('@anima/client test: (pass) one\n@anima/client test: (fail) two');
  expect([...(log?.querySelectorAll('.line.failure') ?? [])].map((line) => line.textContent)).toEqual(['@anima/client test: (fail) two']);
  expect(target.querySelector('[aria-label="Job log"] header')?.textContent).toContain('2 lines');
});
