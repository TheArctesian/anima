/**
 * The request deadline for Headscale steps (H1): a step that changes Headscale is refused,
 * not started, when it could not finish before the deadline, so the 45 s process limit never
 * cuts a change off half way. Reading steps still run. Pure; the clock is a parameter.
 */

import { StepError, type Runner } from './settle.ts';
import { LIST_STEPS, USERS_STEP, type Step } from './verbs.ts';

/** The longest a Headscale step may take; main.ts kills the docker CLI after it. */
export const STEP_MS = 8_000;

const READS = new Set([...LIST_STEPS, USERS_STEP].map((step) => step.join(' ')));

/** Whether the step changes Headscale (every step but the lists). */
export function changes(step: Step): boolean {
  return !READS.has(step.join(' '));
}

/** `runner`, refusing a changing step that could run past `deadline` (ms since the epoch). */
export function withDeadline(runner: Runner, deadline: number, clock: () => number): Runner {
  return (step) => {
    if (changes(step) && clock() + STEP_MS > deadline) return Promise.reject(new StepError('the request deadline is too near to start a change'));
    return runner(step);
  };
}
