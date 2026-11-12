import { flushSync, mount, tick, unmount } from 'svelte';
import { vi } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { installAnimations } from '$lib/forms/animations.fixture';
import { metricsApi } from '$lib/metrics/api';
import type { ThisComputerShell } from '$lib/desktop/agent/shell';
import AgentPanel from '../AgentPanel.svelte';
import type { AgentCalls, ClipboardSink } from '../install.svelte';
import type { AgentRelease } from '../release';

export const CODE = 'anima_enrol_AbCdEfGhIjKlMnOpQrStUv';
const START = new Date('2026-11-09T10:00:00Z');
export const RELEASE: AgentRelease = { version: '1.2.3', sha256: { 'linux-x86_64': 'a'.repeat(64), 'macos-aarch64': 'b'.repeat(64) } };

let app: ReturnType<typeof mount> | undefined;
let clock = START;

export async function settle(): Promise<void> {
  for (let turn = 0; turn < 8; turn += 1) await tick();
  flushSync();
}

/** Mounts the panel for device d1 on the hub's origin, with the fixture's clock. */
/** Where the panel runs: the device's host, and the desktop shell (`null`, a browser, by default). */
export interface Place {
  readonly host?: string;
  readonly shell?: ThisComputerShell | null;
}

interface Setup extends Place {
  readonly release: AgentRelease | null;
  readonly clipboard: ClipboardSink;
}

function mountPanel(calls: AgentCalls, source: 'NONE' | 'AGENT', setup: Setup): void {
  const { release, clipboard, host = '100.64.0.9', shell = null } = setup;
  app = mount(AgentPanel, {
    target: document.body,
    props: { deviceId: 'd1', deviceName: 'nas', host, source, calls, origin: 'https://hub.test', now: () => clock, release, clipboard, shell },
  });
}

/** Mounts a panel whose `call` waits; returns the function that answers it with `value`. */
export async function showPending(call: 'agentStatus' | 'enrolAgent', source: 'NONE' | 'AGENT'): Promise<(value: unknown) => Promise<void>> {
  let answer: (value: unknown) => void = () => undefined;
  const calls = { ...metricsApi(fakeHub({}).hub), [call]: () => new Promise<never>((resolve) => { answer = resolve as typeof answer; }) };
  mountPanel(calls, source, { release: null, clipboard: { writeText: () => Promise.resolve() } });
  await settle();
  return async (value) => {
    answer(value);
    await settle();
  };
}

export interface Shown {
  readonly fake: ReturnType<typeof fakeHub>;
  readonly copied: string[];
}

export async function show(
  answers: Record<string, unknown>,
  props: { source?: 'NONE' | 'AGENT'; release?: AgentRelease | null; copyFails?: boolean; enrolAgent?: AgentCalls['enrolAgent'] } & Place = {},
): Promise<Shown> {
  const fake = fakeHub(answers);
  const copied: string[] = [];
  const clipboard = {
    writeText: (text: string) => (props.copyFails === true ? Promise.reject(new Error('denied')) : (copied.push(text), Promise.resolve())),
  };
  const calls = metricsApi(fake.hub);
  mountPanel({ ...calls, enrolAgent: props.enrolAgent ?? calls.enrolAgent }, props.source ?? 'NONE', {
    release: props.release === undefined ? RELEASE : props.release,
    clipboard,
    host: props.host,
    shell: props.shell,
  });
  await settle();
  return { fake, copied };
}

export function button(label: string, within = 'body'): HTMLButtonElement {
  const found = [...document.querySelectorAll<HTMLButtonElement>(`${within} button`)].find((each) => each.textContent.trim() === label);
  if (!found) throw new Error(`no button ${label}`);
  return found;
}

export async function click(label: string, within = 'body'): Promise<void> {
  button(label, within).click();
  await settle();
}

export const text = () => document.body.textContent;
export const command = () => document.querySelector('pre')?.textContent ?? '';
export const enrolment = (code = CODE) => ({ EnrolAgent: { enrolAgent: { code, expiresAt: '2026-11-09T10:15:00Z' } } });
export const ENROLMENT = enrolment();

/** Moves the injected clock and lets the panel's one-second timer run. */
export async function pass(seconds: number): Promise<void> {
  clock = new Date(clock.getTime() + seconds * 1000);
  await vi.advanceTimersByTimeAsync(seconds * 1000);
  await settle();
}

/** Call from `beforeEach`. */
export function setUp(): void {
  installAnimations();
  clock = START;
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval', 'setTimeout', 'clearTimeout'] });
  Object.defineProperty(navigator, 'platform', { configurable: true, value: 'Linux x86_64' });
}

/** Call from `afterEach`. */
export async function tearDown(): Promise<void> {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
  vi.useRealTimers();
  localStorage.clear();
}

