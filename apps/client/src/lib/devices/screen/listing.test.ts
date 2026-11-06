import { afterEach, expect, it } from 'vitest';
import { DEFAULT_QUERY } from '../query';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import { heldHub } from './held.fixture';
import { Listing } from './listing.svelte';
import { device } from './screen.fixture.svelte';

const settle = async () => {
  for (let round = 0; round < 6; round += 1) await Promise.resolve();
};

function listing() {
  const held = heldHub();
  return { held, listing: new Listing(held.hub, manualClock().clock) };
}

let open: Listing | undefined;

afterEach(() => {
  open?.stop();
  open = undefined;
});

it('DEV-001: stop, then start and load with the same query, fetches and shows the list again', async () => {
  const { held, listing: screen } = listing();
  open = screen;
  screen.start();
  void screen.load(DEFAULT_QUERY);
  held.held[0]?.release([device('a')]);
  await settle();
  screen.stop();
  screen.start();
  void screen.load(DEFAULT_QUERY);
  await settle();
  expect(held.held).toHaveLength(2);
  held.held[1]?.release([device('b')]);
  await settle();
  expect(screen.list?.devices.map((item) => item.id)).toEqual(['b']);
});

it('DEV-001: an answer that is in flight when the page stops shows nothing', async () => {
  const { held, listing: screen } = listing();
  open = screen;
  screen.start();
  void screen.load(DEFAULT_QUERY);
  screen.stop();
  held.held[0]?.release([device('a')]);
  await settle();
  expect(screen.list).toBeNull();
});
