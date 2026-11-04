import { expect, it } from 'vitest';
import { ambientMotionAllowed } from './ambient';

it.each([
  ['visible', false, true],
  ['hidden', false, false],
  ['visible', true, false],
  ['hidden', true, false],
] as const)('MOT-011: a %s window with reduced motion %s allows ambient motion: %s', (visibility, reduced, allowed) => {
  expect(ambientMotionAllowed(visibility, reduced)).toBe(allowed);
});
