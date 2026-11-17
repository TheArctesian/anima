// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import type { CardDevice } from '../api';
import { device } from '../screen/screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';
import PhoneRows from './PhoneRows.svelte';

endTransitionsAtOnce();

const KEYED = { enabled: true, hostKey: { fingerprintSha256: 'SHA256:abc' } };
const GROUPED = { sort: 'STATUS', statuses: [] } as const;

let app: ReturnType<typeof mount> | undefined;
afterEach(() => {
  if (app !== undefined) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function show(devices: CardDevice[], connected: string[] = []): HTMLElement {
  app = mount(PhoneRows, {
    target: document.body,
    props: {
      devices,
      beats: new Map(),
      query: GROUPED,
      onConnect: (shown: CardDevice, protocol: string) => {
        connected.push(`${shown.name} ${protocol}`);
      },
    },
  });
  flushSync();
  return document.body;
}

it('MOB-015: a row names its device and status, and opens the device page', () => {
  const page = show([device('a', { name: 'nas', status: 'DOWN' }), device('b', { name: 'pi', status: 'UP' })]);
  const rows = [...page.querySelectorAll('.dev')];
  expect(rows.map((row) => row.querySelector('.name')?.textContent)).toEqual(['nas', 'pi']);
  expect(rows.map((row) => row.querySelector('.line')?.textContent.replace(/\s+/g, ' ').trim().split(' ')[0])).toEqual(['Down', 'Up']);
  expect(rows[0]?.querySelector('a')?.getAttribute('href')).toBe('/devices/a');
});

it('D18: in the order of the hub, the devices that need a look have their own title and an edge', () => {
  const page = show([device('a', { name: 'nas', status: 'DOWN' }), device('b', { name: 'pi', status: 'UP' })]);
  expect([...page.querySelectorAll('h2')].map((title) => title.firstChild?.textContent)).toEqual(['Needs a look', 'Up']);
  expect(page.querySelector('.dev')?.getAttribute('data-edge')).toBe('down');
});

it('mockup 4: the button opens a session with the first protocol that the device has; a device with none has no button', () => {
  const connected: string[] = [];
  const page = show([device('a', { name: 'nas', ssh: KEYED }), device('b', { name: 'pi' })], connected);
  const buttons = [...page.querySelectorAll('button')];
  expect(buttons.map((button) => button.getAttribute('aria-label'))).toHaveLength(1);
  buttons[0]?.click();
  expect(connected).toEqual(['nas SSH']);
});
