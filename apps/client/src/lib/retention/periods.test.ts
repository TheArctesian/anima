import { expect, it } from 'vitest';
import { checkDays, inPageOrder, SUBJECTS } from './periods';
import { FIXTURE_PERIODS } from './retention.fixture';

const events = { minDays: 30, maxDays: 365 };

it('HUB-005: whole numbers inside the bounds pass, bounds included', () => {
  expect(checkDays('30', events)).toEqual({ ok: true, days: 30 });
  expect(checkDays(' 365 ', events)).toEqual({ ok: true, days: 365 });
});

it('HUB-005: a number outside the bounds is refused with the bounds in the message', () => {
  for (const text of ['29', '366', '-1', '0']) {
    expect(checkDays(text, events)).toEqual({ ok: false, message: 'Enter a number of days from 30 to 365.' });
  }
});

it('HUB-005: a value that is not a whole number is refused', () => {
  for (const text of ['', 'abc', '1.5', '1e2', '30 days']) {
    expect(checkDays(text, events)).toEqual({ ok: false, message: 'Enter a whole number of days.' });
  }
});

it('D-25: the four subjects are listed in page order whatever order the hub answers in', () => {
  expect(inPageOrder([...FIXTURE_PERIODS].reverse()).map((period) => period.subject)).toEqual(SUBJECTS.map((each) => each.subject));
});
