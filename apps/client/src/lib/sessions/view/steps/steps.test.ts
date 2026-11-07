import { expect, it } from 'vitest';
import { connectionSteps, wakeLine, type Progress } from './steps';

const at = (progress: Partial<Progress>): string[] =>
  connectionSteps({ ticketed: false, waking: false, opened: false, connected: false, stopped: false, ...progress }).map(
    (step) => `${step.name}:${step.mark}`,
  );

it.each([
  [{}, ['request:active', 'gateway:waiting', 'device:waiting', 'ready:waiting']],
  [{ ticketed: true }, ['request:done', 'gateway:active', 'device:waiting', 'ready:waiting']],
  [{ ticketed: true, opened: true }, ['request:done', 'gateway:done', 'device:active', 'ready:waiting']],
  [{ ticketed: true, opened: true, connected: true }, ['request:done', 'gateway:done', 'device:done', 'ready:done']],
  [{ stopped: true }, ['request:failed', 'gateway:waiting', 'device:waiting', 'ready:waiting']],
  [{ ticketed: true, stopped: true }, ['request:done', 'gateway:failed', 'device:waiting', 'ready:waiting']],
  [{ ticketed: true, opened: true, stopped: true }, ['request:done', 'gateway:done', 'device:failed', 'ready:waiting']],
  [{ ticketed: true, opened: true, connected: true, stopped: true }, ['request:done', 'gateway:done', 'device:done', 'ready:done']],
])('5.7.3 connection steps: progress %o marks %o', (progress, marks) => {
  expect(at(progress)).toEqual(marks);
});

const wake = (progress: Partial<Progress>, name = 'web-01') =>
  wakeLine({ ticketed: true, waking: true, opened: false, connected: false, stopped: false, ...progress }, name);

it.each([
  [{}, 'active'],
  [{ opened: true }, 'done'],
  [{ stopped: true }, 'failed'],
] as const)('RA-031, K4 §1: the wake line for progress %o is marked %s', (progress, mark) => {
  expect(wake(progress)).toEqual({ label: 'Waking web-01', mark });
});

it('RA-031: the wake line names the device as plain text', () => {
  expect(wake({}, '<b>web-01</b>')?.label).toBe('Waking <b>web-01</b>');
});

it('RA-031: without a wake packet there is no wake line, and the four steps are the same', () => {
  expect(wake({ waking: false })).toBeUndefined();
  expect(at({ waking: true, ticketed: true })).toEqual(at({ ticketed: true }));
});

it('MOT-007, M-3: the wake line has the fixed icon cell of a step, so done and active put the label in one place', async () => {
  const { readFileSync } = await import('node:fs');
  const css = readFileSync(`${import.meta.dirname}/ConnectionSteps.svelte`, 'utf8');
  expect(css).toMatch(/\.wake \{[^}]*grid-template-columns: var\(--icon-md\)/);
  expect(css).toMatch(/\.wake > :global\(\.icon\) \{[^}]*justify-self: center/);
});
