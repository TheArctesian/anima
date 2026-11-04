import { expect, it } from 'vitest';
import { connectionSteps, type Progress } from './steps';

const at = (progress: Partial<Progress>): string[] =>
  connectionSteps({ ticketed: false, opened: false, connected: false, stopped: false, ...progress }).map(
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
