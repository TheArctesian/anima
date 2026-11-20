// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../../motion/engine.fixture';
import { duration } from '../../motion/tokens';
import { hostLine, shortHost, statusLabel } from './card';
import { shown } from './status.fixture.svelte';
import StatusEdge from '../status/StatusEdge.svelte';
import StatusBadge from '../status/StatusBadge.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const preference = vi.hoisted(() => ({ reduced: false }));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => preference }));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  runs.length = 0;
  preference.reduced = false;
  shown.status = 'UP';
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('6.1 card: a status is text, not only colour', () => {
  expect(statusLabel('MAINTENANCE')).toBe('Maintenance');
  expect(statusLabel('UP')).toBe('Up');
});

it('a long IPv6 address keeps its first two and its last two groups; each other host is whole', () => {
  expect(shortHost('fd41:4e49:4d41:1e00:8c2f:11ff:fe6a:90b3')).toBe('fd41:4e49:…:fe6a:90b3');
  expect(shortHost('fe80::1')).toBe('fe80::1');
  expect(shortHost('10.40.0.21')).toBe('10.40.0.21');
  expect(shortHost('nas.annex.example')).toBe('nas.annex.example');
});

it('D-7: an IP shows next to a host name, not next to an IP host, and not when unknown', () => {
  expect(hostLine({ host: 'nas.lan', ip: '10.0.0.2' })).toEqual({ host: 'nas.lan', ip: '10.0.0.2' });
  expect(hostLine({ host: '10.0.0.2', ip: '10.0.0.2' }).ip).toBeNull();
  expect(hostLine({ host: 'fe80::1', ip: 'fe80::1' }).ip).toBeNull();
  expect(hostLine({ host: 'nas.lan', ip: null }).ip).toBeNull();
});

it('Status change: the badge shows the new status text and carries data-device-id', async () => {
  app = mount(StatusBadge, {
    target: document.body,
    props: {
      get status() {
        return shown.status;
      },
      deviceId: 'd1',
    },
  });
  flushSync();
  expect(document.querySelector('[data-device-id="d1"]')?.textContent.trim()).toBe('Up');
  shown.status = 'DOWN';
  flushSync();
  await vi.waitFor(() => {
    expect(document.querySelector('.slot')?.textContent.trim()).toBe('Down');
  });
});

it.each([false, true])('Status change: the edge highlight fades over duration-highlight (reduced %s)', (reduced) => {
  preference.reduced = reduced;
  app = mount(StatusEdge, {
    target: document.body,
    props: {
      get status() {
        return shown.status;
      },
    },
  });
  flushSync();
  expect(runs).toEqual([]);
  shown.status = 'DOWN';
  flushSync();
  expect(runs.at(-1)?.keyframes.at(-1)).toBe(0);
  expect(runs.at(-1)?.options).toMatchObject({ duration: duration.highlight / 1000 });
});

it('D-7: an empty ip is no IP', () => {
  expect(hostLine({ host: 'nas.lan', ip: '' }).ip).toBeNull();
});
