import { describe, expect, it } from 'vitest';
import { outputFlow } from './flow';

describe('output flow control (K4 §8.5, availability)', () => {
  it('K4 §8.5: output under the high-water mark is drawn', () => {
    const flow = outputFlow(100, 40);
    expect([flow.admit(60), flow.admit(40)]).toEqual(['draw', 'draw']);
  });

  it('K4 §8.5: above the mark output is skipped, and the operator is told once per episode', () => {
    const flow = outputFlow(100, 40);
    flow.admit(90);
    expect([flow.admit(20), flow.admit(20), flow.admit(1)]).toEqual(['skip-and-tell', 'skip', 'skip']);
  });

  it('K4 §8.5: drawing resumes only when the backlog is down to the low-water mark, and a later episode tells again', () => {
    const flow = outputFlow(100, 40);
    flow.admit(90);
    expect(flow.admit(20)).toBe('skip-and-tell');
    flow.drawn(40);
    expect(flow.admit(5)).toBe('skip');
    flow.drawn(10);
    expect(flow.admit(5)).toBe('draw');
    expect(flow.admit(95)).toBe('skip-and-tell');
  });

  it('K4 §8.5: a skipped piece is never counted as waiting', () => {
    const flow = outputFlow(100, 40);
    flow.admit(90);
    flow.admit(50);
    flow.drawn(90);
    expect(flow.admit(100)).toBe('draw');
  });
});
