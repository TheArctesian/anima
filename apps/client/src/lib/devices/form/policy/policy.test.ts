import { expect, it } from 'vitest';
import { createInput, updateInput } from '../input';
import { formValues } from '../values';
import { SAVED } from '../form/form.fixture';
import { DEFAULT_POLICY, LIMIT_SENTENCE, createPolicyInput, limitProblem, updatePolicyInput } from './policy';

const stored = (sessionLimit: string, recordSessions = false) => ({ sessionLimit, recordSessions });

it('RA-036: 101, 0 on create and 2.5 are refused with one sentence; 1, 100 and empty are not', () => {
  for (const text of ['101', '0', '2.5', 'abc', '-1', '1e2']) expect(limitProblem(stored(text), true), text).toBe(LIMIT_SENTENCE);
  for (const text of ['1', '100', '', ' 7 ']) expect(limitProblem(stored(text), true), text).toBeNull();
  expect(limitProblem(stored('101'), false)).toBe(LIMIT_SENTENCE);
  expect(limitProblem(stored('2.5'), false)).toBe(LIMIT_SENTENCE);
});

it('RA-036: on update 0 is accepted, because it clears the limit', () => {
  expect(limitProblem(stored('0'), false)).toBeNull();
});

it('RA-036: an emptied or zero limit sends 0 to clear a stored limit; an untouched field sends nothing', () => {
  expect(updatePolicyInput(stored('5'), stored(''))).toEqual({ sessionLimit: 0 });
  expect(updatePolicyInput(stored('5'), stored('0'))).toEqual({ sessionLimit: 0 });
  expect(updatePolicyInput(stored('5'), stored('5'))).toEqual({});
  expect(updatePolicyInput(stored(''), stored(''))).toEqual({});
  expect(updatePolicyInput(stored(''), stored('0'))).toEqual({});
  expect(updatePolicyInput(stored('5'), stored('12'))).toEqual({ sessionLimit: 12 });
});

it('RA-040: recording is sent on update only when it changed', () => {
  expect(updatePolicyInput(stored('', false), stored('', true))).toEqual({ recordSessions: true });
  expect(updatePolicyInput(stored('', true), stored('', false))).toEqual({ recordSessions: false });
  expect(updatePolicyInput(stored('', true), stored('', true))).toEqual({});
});

it('RA-040: a new device is not recorded, and an untouched form sends neither field on create', () => {
  expect(DEFAULT_POLICY.recordSessions).toBe(false);
  expect(createPolicyInput(DEFAULT_POLICY)).toEqual({});
  const input = createInput({ ...formValues(null), name: 'n', host: 'h.home.arpa', policy: DEFAULT_POLICY }, 's1');
  expect('sessionLimit' in input || 'recordSessions' in input).toBe(false);
});

it('RA-036, RA-040: a new device sends a typed limit and a ticked recording', () => {
  expect(createPolicyInput(stored('3', true))).toEqual({ sessionLimit: 3, recordSessions: true });
});

it('F-3: the device update never carries the policy; it has its own document', () => {
  const before = formValues(SAVED);
  expect(updateInput(before, { ...before, policy: stored('9', true) })).toEqual({});
});
