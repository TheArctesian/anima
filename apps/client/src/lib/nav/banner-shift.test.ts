// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { runs } from '../motion/engine.fixture';
import { spring } from '../motion/tokens';
import { watchHubBanner } from './banner-shift';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../motion/engine.fixture')).fakeRun,
}));

const ROOM = 80;

/** A page with a bar and a main whose children sit ROOM lower while the root has `data-hub-banner`. */
function page() {
  document.body.innerHTML = `
    <nav data-shifts-with-banner><a id="item">Devices</a><div id="host" data-holds-banner></div></nav>
    <main data-shifts-with-banner>
      <div id="stage"><div id="flow"></div><div id="still"></div><div id="shell"><div id="panel" style="position: fixed"></div><div id="beside"></div></div></div>
      <div id="tabs"></div>
    </main>`;
  const place = (id: string, moves = true) => {
    const node = document.getElementById(id) as HTMLElement;
    node.getBoundingClientRect = () => {
      const [x = 0, y = 0] = node.style.translate.split(/\s+/).filter((part) => part !== '').map((part) => Number.parseFloat(part));
      const base = moves && document.documentElement.hasAttribute('data-hub-banner') ? ROOM : 0;
      return new DOMRect(x, base + y, 100, 40);
    };
    return node;
  };
  for (const id of ['flow', 'shell', 'panel', 'beside', 'tabs']) place(id);
  place('still', false);
  // The stage holds a fixed element, so it is not moved itself: its in-flow children are.
  return { item: place('item'), host: place('host'), stage: place('stage'), el: (id: string) => document.getElementById(id) as HTMLElement };
}

beforeEach(() => {
  runs.length = 0;
});

afterEach(() => {
  document.documentElement.removeAttribute('data-hub-banner');
  document.body.innerHTML = '';
});

it('MOT-007: the first state is set with no motion, since nothing was on screen before', () => {
  page();
  watchHubBanner(document.documentElement, { reduced: false }).show(true);
  expect(document.documentElement.hasAttribute('data-hub-banner')).toBe(true);
  expect(runs).toEqual([]);
});

const lifted = (node: HTMLElement) => node.style.translate;
const later = () => new Promise((done) => setTimeout(done));

it('MOT-002: when the strip appears the bar items and the page move from where they were on spring-default', () => {
  const { item, el } = page();
  const stage = el('flow');
  const strip = watchHubBanner(document.documentElement, { reduced: false });
  strip.show(false);
  strip.show(true);
  expect(document.documentElement.hasAttribute('data-hub-banner')).toBe(true);
  // The item and the stage each start ROOM above their new place and spring to it.
  expect([lifted(item), lifted(stage)]).toEqual(['0px -80px', '0px -80px']);
  expect(runs.filter((run) => run.keyframes.at(-1) === 0)).not.toHaveLength(0);
  expect(runs.at(-1)?.options).toMatchObject({ stiffness: spring.default.stiffness, damping: spring.default.damping });
});

it('NET-004: the element that holds the fixed strip never gets a translate, which would become its containing block', () => {
  const { host } = page();
  const strip = watchHubBanner(document.documentElement, { reduced: false });
  strip.show(false);
  strip.show(true);
  expect(host.style.translate).toBe('');
});

it('MOT-002: a change mid-flight starts from the place on screen, not from the layout', () => {
  const { item, el } = page();
  const stage = el('flow');
  const strip = watchHubBanner(document.documentElement, { reduced: false });
  strip.show(false);
  strip.show(true);
  for (const run of runs) run.frame(run.keyframes[0] === 0 ? 0 : -ROOM / 2);
  strip.show(false);
  // Halfway through, the strip goes: the elements are ROOM / 2 below where they now belong.
  expect([lifted(item), lifted(stage)]).toEqual(['0px 40px', '0px 40px']);
});

it('MOT-040: under reduced motion the page is in place at once and nothing is left translated', async () => {
  const { el } = page();
  const stage = el('flow');
  const strip = watchHubBanner(document.documentElement, { reduced: true });
  strip.show(false);
  strip.show(true);
  await later();
  expect(runs).toEqual([]);
  expect(stage.getBoundingClientRect().top).toBe(ROOM);
  expect(stage.style.translate).toBe('');
});

it('a flight that has arrived leaves no translate on the page, so a fixed modal inside it is placed by the window', async () => {
  const { el } = page();
  const stage = el('flow');
  const strip = watchHubBanner(document.documentElement, { reduced: false });
  strip.show(false);
  strip.show(true);
  for (const run of runs) run.finish();
  await later();
  expect(stage.style.translate).toBe('');
});

it('stop removes the attribute', () => {
  page();
  const strip = watchHubBanner(document.documentElement, { reduced: false });
  strip.show(true);
  strip.stop();
  expect(document.documentElement.hasAttribute('data-hub-banner')).toBe(false);
});

it('NET-004: an element with a fixed element inside is not translated, its in-flow children are; the fixed element stays', () => {
  const { stage, el } = page();
  const strip = watchHubBanner(document.documentElement, { reduced: false });
  strip.show(false);
  strip.show(true);
  expect([stage, el('shell'), el('panel')].map((node) => node.style.translate)).toEqual(['', '', '']);
  expect([el('flow'), el('beside'), el('tabs')].map((node) => node.style.translate)).toEqual(['0px -80px', '0px -80px', '0px -80px']);
});

it('MOT-002: an element that does not change place gets no translate', () => {
  const { el } = page();
  const strip = watchHubBanner(document.documentElement, { reduced: false });
  strip.show(false);
  strip.show(true);
  expect(el('still').style.translate).toBe('');
});
