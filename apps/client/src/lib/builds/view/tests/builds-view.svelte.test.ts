// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import BuildsView from '../BuildsView.svelte';
import { COMMIT, fakeHub, job, ok, refused, release, settled, type Answers } from './builds-view.fixture';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

const RELEASES = [
  release('rel-1', '0.1.12', '2026-11-10T10:00:00Z', [
    job({ id: 'job-a', releaseId: 'rel-1', state: 'PUBLISHED', builderName: 'mac-mini', attempts: 1 }),
    job({ id: 'job-b', releaseId: 'rel-1', target: 'DARWIN_UNIVERSAL', state: 'FAILED', failureReason: 'builder stopped 3 times', attempts: 3 }),
  ]),
  release('rel-2', '0.1.13', '2026-11-11T10:00:00Z', [job({ id: 'job-c', releaseId: 'rel-2' })]),
];

const PLATFORM = { os: 'LINUX', arch: 'X86_64' } as const;

beforeEach(() => {
  installAnimations();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function open(answers: Answers = {}): Promise<HTMLElement> {
  hub = fakeHub({
    Releases: ok({ releases: { edges: RELEASES.map((node) => ({ node })) } }),
    NextReleaseVersion: ok({ nextReleaseVersion: '0.1.14' }),
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

function named<T extends HTMLElement>(target: HTMLElement, selector: string, name: string): T {
  const found = [...target.querySelectorAll<T>(selector)].find((element) => element.getAttribute('aria-label') === name);
  if (found === undefined) throw new Error(`no ${selector} named ${name}`);
  return found;
}

function input(target: HTMLElement, label: string): HTMLInputElement {
  const found = [...target.querySelectorAll('label')].find((element) => element.textContent.trim() === label);
  const control = found?.control ?? found?.querySelector('input') ?? null;
  if (!(control instanceof HTMLInputElement)) throw new Error(`no field ${label}`);
  return control;
}

function type(control: HTMLInputElement, value: string): void {
  control.value = value;
  control.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

async function submit(target: HTMLElement): Promise<void> {
  target.querySelector('form')?.dispatchEvent(new SubmitEvent('submit', { bubbles: true, cancelable: true }));
  flushSync();
  await settled();
}

it('BLD-080: releases are listed newest first, each with its commit, its operator and its jobs', async () => {
  const target = await open();
  const versions = [...target.querySelectorAll('.releases > li > h3')].map((heading) => heading.textContent);
  expect(versions).toEqual(['0.1.13', '0.1.12']);
  const older = named(target, 'li', 'Release 0.1.12');
  expect(older.textContent).toContain('0123456');
  expect(older.textContent).toContain('operator');
  expect(older.textContent).toContain('Published');
  expect(older.textContent).toContain('mac-mini');
  expect(older.textContent).toContain('1 attempt');
  expect(older.textContent).toContain('3 attempts');
});

it('BLD-080: a failed job shows its failure reason', async () => {
  const target = await open();
  expect(named(target, 'li', 'Release 0.1.12').textContent).toContain('builder stopped 3 times');
});

it('BLD-007: a queued job with no online builder says it waits for one', async () => {
  const target = await open();
  expect(target.textContent).toContain('Waiting for a linux-x86_64 builder');
});

it('BLD-007: a queued job says no more once a builder for its target is online', async () => {
  const target = await open({ Builders: ok({ builders: [{ id: 'b', name: 'box', online: true, platform: PLATFORM }] }) });
  expect(target.textContent).not.toContain('Waiting for a');
});

it('BLD-001: the version is prefilled from the hub and the release is created with the commit and the targets', async () => {
  const target = await open({ CreateRelease: ok({ createRelease: { id: 'rel-3' } }) });
  const version = input(target, 'Version');
  expect(version.value).toBe('0.1.14');
  type(input(target, 'Commit'), COMMIT);
  await submit(target);
  const create = hub.sent.find((request) => request.operation === 'CreateRelease');
  expect(create?.variables).toEqual({ version: '0.1.14', commit: COMMIT, targets: ['LINUX_X86_64', 'DARWIN_UNIVERSAL'] });
});

it('BLD-001: the button names the version, and the operator can change the version and the targets', async () => {
  const target = await open({ CreateRelease: ok({ createRelease: { id: 'rel-3' } }) });
  type(input(target, 'Version'), '0.2.0');
  type(input(target, 'Commit'), COMMIT);
  expect(target.querySelector<HTMLButtonElement>('form button[type="submit"]')?.textContent.trim()).toBe('Release 0.2.0');
  input(target, 'darwin-universal').click();
  flushSync();
  await submit(target);
  const create = hub.sent.find((request) => request.operation === 'CreateRelease');
  expect(create?.variables).toEqual({ version: '0.2.0', commit: COMMIT, targets: ['LINUX_X86_64'] });
});

it('BLD-001: a commit that is not 40 hex digits is refused before the hub is asked', async () => {
  const target = await open();
  type(input(target, 'Commit'), 'main');
  await submit(target);
  expect(target.textContent).toContain('A commit is 40 lower-case hex digits.');
  expect(hub.sent.some((request) => request.operation === 'CreateRelease')).toBe(false);
});

it('BLD-002: the hub refusal is shown in the form, with no dialog', async () => {
  const target = await open({ CreateRelease: refused('linux-x86_64 has published 0.1.14. Use a later version.') });
  type(input(target, 'Commit'), COMMIT);
  await submit(target);
  expect(target.querySelector('form')?.textContent).toContain('linux-x86_64 has published 0.1.14. Use a later version.');
  expect(document.querySelector('dialog, [role="dialog"]')).toBeNull();
});

it('BLD-008: only a queued job has Cancel, and it calls cancelBuildJob for that job', async () => {
  const target = await open({ CancelBuildJob: ok({ cancelBuildJob: { id: 'job-c', state: 'CANCELLED' } }) });
  const cancels = [...target.querySelectorAll('button')].filter((button) => button.textContent.trim() === 'Cancel');
  expect(cancels.map((button) => button.getAttribute('aria-label'))).toEqual(['Cancel linux-x86_64 job of 0.1.13']);
  cancels[0]?.click();
  await settled();
  expect(hub.sent.find((request) => request.operation === 'CancelBuildJob')?.variables).toEqual({ id: 'job-c' });
});

it('BLD-081: selecting a job shows its log, and lines pushed by the subscription join it, cleaned again', async () => {
  const target = await open({
    BuildJobLog: ok({ buildJobLog: [{ sequence: 1, text: 'cargo build' }] }),
  });
  named<HTMLButtonElement>(target, 'button', 'Show log of linux-x86_64 job of 0.1.13').click();
  await settled();
  expect(target.querySelector('pre')?.textContent).toBe('cargo build');
  expect(hub.sent.find((request) => request.operation === 'BuildJobLog')?.variables).toMatchObject({ id: 'job-c', after: null });
  hub.push({
    job: job({ id: 'job-c', releaseId: 'rel-2', state: 'BUILDING' }),
    logLines: [{ sequence: 2, text: 'TAURI_SIGNING_PRIVATE_KEY=hunter2hunter2' }, { sequence: 1, text: 'cargo build' }],
  });
  flushSync();
  expect(target.querySelector('pre')?.textContent).toBe('cargo build\nTAURI_SIGNING_PRIVATE_KEY=[redacted]');
  expect(named(target, 'li', 'Release 0.1.13').textContent).toContain('Building');
});

it('BLD-080: when the releases cannot be loaded the view says so and offers to try again', async () => {
  const target = await open({ Releases: { ok: false, error: { code: 'unreachable', message: 'The hub did not answer.' } } });
  expect(target.textContent).toContain('Anima could not load the releases.');
  expect([...target.querySelectorAll('button')].map((button) => button.textContent.trim())).toContain('Try again');
});
