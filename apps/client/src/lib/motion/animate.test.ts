// @vitest-environment jsdom
import { beforeEach, expect, it, vi } from 'vitest';
import { animate } from './animate';
import { runs } from './engine.fixture';
import { springMotion, tweenMotion } from './options';
import { duration, spring } from './tokens';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('./engine.fixture')).fakeRun,
}));

function element(): HTMLElement {
  const node = document.createElement('div');
  document.body.append(node);
  return node;
}

function styledProperties(node: HTMLElement): string[] {
  return [...node.style].sort();
}

beforeEach(() => {
  runs.length = 0;
});

it('MOT-002: a second motion starts from the value on screen, not the end of the first', () => {
  const node = element();
  animate(node, { translate: { x: 100 } }, springMotion('default', false));
  const [first] = runs;
  first?.frame(40);
  expect(node.style.translate).toBe('40px 0px');

  animate(node, { translate: { x: 0 } }, springMotion('default', false));
  expect(runs[1]?.keyframes).toEqual([40, 0]);
  expect(first?.stopped).toBe(true);
});

it('MOT-002: the first motion starts from the computed style of the element', () => {
  const node = element();
  node.style.scale = '0.5';
  animate(node, { scale: 1 }, springMotion('default', false));
  expect(runs.map((run) => run.keyframes)).toEqual([
    [0.5, 1],
    [0.5, 1],
  ]);
});

it('MOT-035: moving one axis keeps the other axis where it is on screen', () => {
  const node = element();
  animate(node, { translate: { x: 10 } }, { type: 'instant' });
  node.style.translate = '10px 30px';
  animate(node, { translate: { x: 0 } }, { type: 'instant' });
  expect(node.style.translate).toBe('0px 30px');
});

it('MOT-006: only translate, scale and opacity are written', () => {
  const node = element();
  animate(node, { translate: { x: 4, y: 8 }, scale: { y: 0.5 }, opacity: 0 }, { type: 'instant' });
  expect(styledProperties(node)).toEqual(['opacity', 'scale', 'translate']);
  expect(node.style.translate).toBe('4px 8px');
  expect(node.style.scale).toBe('1 0.5');
});

it('MOT-006: other properties are refused by the type', () => {
  const node = element();
  // @ts-expect-error -- MOT-006: width is not an animatable property
  animate(node, { width: 10 }, { type: 'instant' });
  expect(styledProperties(node)).toEqual([]);
});

it('MOT-042: instant motion is on screen before animate returns', async () => {
  const node = element();
  const playback = animate(node, { opacity: 0 }, springMotion('move', true));
  expect(node.style.opacity).toBe('0');
  expect(runs).toEqual([]);
  await expect(playback.finished).resolves.toBeUndefined();
});

it('MOT-042: an interrupted or stopped motion settles, so nothing waits for it', async () => {
  const node = element();
  const interrupted = animate(node, { opacity: 0 }, tweenMotion('fast', 'exit', false));
  animate(node, { opacity: 1 }, tweenMotion('fast', 'enter', false));
  await expect(interrupted.finished).resolves.toBeUndefined();

  const stopped = animate(node, { opacity: 0 }, tweenMotion('fast', 'exit', false));
  stopped.stop();
  await expect(stopped.finished).resolves.toBeUndefined();
});

it('MOT-003/031: a spring runs with the token stiffness, damping and the release velocity', () => {
  animate(element(), { translate: { x: 0 } }, springMotion('momentum', false, 1200));
  expect(runs[0]?.options).toEqual({
    type: 'spring',
    stiffness: spring.momentum.stiffness,
    damping: spring.momentum.damping,
    mass: 1,
    velocity: 1200,
  });
});

it('MOT-003: an opacity tween runs for the token duration in seconds', () => {
  animate(element(), { opacity: 0 }, tweenMotion('fast', 'exit', false));
  expect(runs[0]?.options).toMatchObject({ duration: duration.fast / 1000 });
});

it('stopping a superseded playback leaves the newer motion running', () => {
  const node = element();
  const older = animate(node, { opacity: 0 }, tweenMotion('fast', 'exit', false));
  animate(node, { opacity: 1 }, tweenMotion('fast', 'enter', false));
  older.stop();
  expect(runs[1]?.stopped).toBe(false);
});
