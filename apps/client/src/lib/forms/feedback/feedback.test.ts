import { expect, it } from 'vitest';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import { duration, threshold } from '$lib/motion/tokens';
import { actionFeedback, type ActionState } from './feedback';

function feedback() {
  const time = manualClock();
  const states: ActionState[] = [];
  return { time, states, button: actionFeedback(time.clock, duration.highlight, (state) => states.push(state)) };
}

it('Feedback: work that ends before threshold-loading never shows the pending label', () => {
  const { time, states, button } = feedback();
  button.begin();
  time.advance(threshold.loading - 1);
  button.succeed();
  expect(states).toEqual(['done']);
});

it('Feedback: slow work shows the pending label, then done, then the label again after duration-highlight', () => {
  const { time, states, button } = feedback();
  button.begin();
  time.advance(threshold.loading);
  expect(states).toEqual(['pending']);
  button.succeed();
  time.advance(duration.highlight - 1);
  expect(states).toEqual(['pending', 'done']);
  time.advance(1);
  expect(states).toEqual(['pending', 'done', 'idle']);
});

it('Feedback: a failure returns the label at once and leaves no wait behind', () => {
  const { time, states, button } = feedback();
  button.begin();
  time.advance(threshold.loading);
  button.fail();
  expect(states).toEqual(['pending', 'idle']);
  expect(time.pending()).toBe(0);
});

it('Feedback: a second press while done starts over without the old hold ending it early', () => {
  const { time, states, button } = feedback();
  button.begin();
  button.succeed();
  time.advance(duration.highlight - 10);
  button.begin();
  time.advance(threshold.loading);
  expect(states).toEqual(['done', 'pending']);
});

it('Feedback: stop cancels every wait', () => {
  const { time, button } = feedback();
  button.begin();
  button.stop();
  expect(time.pending()).toBe(0);
});
