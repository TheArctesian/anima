import { expect, it } from 'vitest';
import { channelValues, defaultChannelIds, differs, notificationsInput, RESEND_SENTENCE, resendProblem } from './channels';

it('NOT-003: a new check starts with the channels that are on by default', () => {
  expect(defaultChannelIds([{ id: 'a', onByDefault: false }, { id: 'b', onByDefault: true }])).toEqual(['b']);
});

it('NOT-005: the resend interval is a whole number from 0; 0 never resends', () => {
  const values = (resend: string) => ({ channelIds: [], resend });
  expect(resendProblem(values('0'))).toBeNull();
  expect(resendProblem(values(' 12 '))).toBeNull();
  for (const bad of ['', '-1', '1.5', 'x', '1e3']) expect(resendProblem(values(bad))).toBe(RESEND_SENTENCE);
});

it('NOT-003: the input sends sorted channel IDs and the interval as a number', () => {
  expect(notificationsInput({ channelIds: ['b', 'a'], resend: '5' })).toEqual({ channelIds: ['a', 'b'], resendEveryBeats: 5 });
});

it('NOT-003: an unchanged section is not different, whatever the order of the channels', () => {
  const stored = { channelIds: ['a', 'b'], resendEveryBeats: 5 };
  expect(differs(stored, channelValues({ channelIds: ['b', 'a'], resendEveryBeats: 5 }))).toBe(false);
  expect(differs(stored, { channelIds: ['a'], resend: '5' })).toBe(true);
  expect(differs(stored, { channelIds: ['a', 'b'], resend: '6' })).toBe(true);
});
