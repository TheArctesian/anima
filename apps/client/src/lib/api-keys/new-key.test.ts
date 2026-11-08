import { describe, expect, it } from 'vitest';
import { isExpired, lastUsedText, scopeLabel } from './listing';
import {
  checkNewKey,
  createFailure,
  DATE_MISSING,
  DATE_PAST,
  DATE_TOO_FAR,
  emptyForm,
  expiryOf,
  latestExpiry,
  NAME_MISSING,
  NAME_TOO_LONG,
  type NewKeyForm,
} from './new-key';
import { NOW } from './api-keys.fixture';

const DAY_MS = 86_400_000;
const HUB_BOUND_MS = 365 * DAY_MS;

function form(changes: Partial<NewKeyForm>): NewKeyForm {
  return { ...emptyForm(), name: 'backup-job', ...changes };
}

describe('the create form (INT-003, D-23)', () => {
  it('D-23: the form starts with scope Read and expiry 90 days', () => {
    expect(emptyForm()).toEqual({ name: '', scope: 'READ', expiry: '90d', date: '' });
  });

  it('INT-003: an empty or blank name is refused', () => {
    for (const name of ['', '   ']) {
      expect(checkNewKey(form({ name }), NOW)).toEqual({ ok: false, problems: [{ field: 'name', message: NAME_MISSING }] });
    }
  });

  it('INT-003: a name of 64 characters is sent and one of 65 is refused, counted in characters', () => {
    expect(checkNewKey(form({ name: 'é'.repeat(64) }), NOW).ok).toBe(true);
    expect(checkNewKey(form({ name: '🔑'.repeat(64) }), NOW).ok).toBe(true);
    expect(checkNewKey(form({ name: 'a'.repeat(65) }), NOW)).toEqual({ ok: false, problems: [{ field: 'name', message: NAME_TOO_LONG }] });
  });

  it('D-23: "On a date" without a date, or with a day that does not exist, is refused as missing', () => {
    for (const date of ['', '2027-02-30', 'tomorrow']) {
      expect(checkNewKey(form({ expiry: 'date', date }), NOW)).toEqual({ ok: false, problems: [{ field: 'expiry', message: DATE_MISSING }] });
    }
  });

  it('D-23: a date that has begun is refused', () => {
    expect(checkNewKey(form({ expiry: 'date', date: '2026-11-01' }), NOW)).toEqual({ ok: false, problems: [{ field: 'expiry', message: DATE_PAST }] });
  });

  it('D-23: a date more than one year away is refused', () => {
    expect(checkNewKey(form({ expiry: 'date', date: '2027-11-09' }), NOW)).toEqual({ ok: false, problems: [{ field: 'expiry', message: DATE_TOO_FAR }] });
  });

});

describe('the expiry bounds (D-23)', () => {
  it('D-23: a date whose day begins exactly now is refused; the latest allowed instant is sent and one millisecond more is not', () => {
    const midnight = new Date(2027, 5, 1);
    const onTheDay = form({ expiry: 'date', date: '2027-06-01' });
    expect(checkNewKey(onTheDay, midnight)).toEqual({ ok: false, problems: [{ field: 'expiry', message: DATE_PAST }] });
    const atTheBound = new Date(midnight.getTime() - (latestExpiry(NOW).getTime() - NOW.getTime()));
    expect(latestExpiry(atTheBound).getTime()).toBe(midnight.getTime());
    expect(checkNewKey(onTheDay, atTheBound).ok).toBe(true);
    expect(checkNewKey(onTheDay, new Date(atTheBound.getTime() - 1))).toEqual({ ok: false, problems: [{ field: 'expiry', message: DATE_TOO_FAR }] });
  });

  it('D-23: every problem is reported at once', () => {
    const checked = checkNewKey(form({ name: '', expiry: 'date', date: '' }), NOW);
    expect(checked.ok ? [] : checked.problems.map((problem) => problem.field)).toEqual(['name', 'expiry']);
  });

  it('INT-003: a valid form gives the trimmed name, the scope and the expiry as an ISO instant', () => {
    expect(checkNewKey(form({ name: '  backup-job ', scope: 'WRITE' }), NOW)).toEqual({
      ok: true,
      input: { name: 'backup-job', scope: 'WRITE', expiresAt: '2027-02-05T12:00:00.000Z' },
    });
    expect(checkNewKey(form({ expiry: 'date', date: '2027-06-01' }), NOW)).toEqual({
      ok: true,
      input: { name: 'backup-job', scope: 'READ', expiresAt: new Date(2027, 5, 1).toISOString() },
    });
  });

  it('D-23: the presets count from the given time: 30 days, 90 days, and "1 year" a few minutes inside 365 days', () => {
    expect(expiryOf({ expiry: '30d', date: '' }, NOW)?.toISOString()).toBe('2026-12-07T12:00:00.000Z');
    expect(expiryOf({ expiry: '90d', date: '' }, NOW)?.toISOString()).toBe('2027-02-05T12:00:00.000Z');
    const year = expiryOf({ expiry: '1y', date: '' }, NOW)?.getTime() ?? 0;
    expect(year - NOW.getTime()).toBeLessThan(HUB_BOUND_MS);
    expect(year - NOW.getTime()).toBeGreaterThan(HUB_BOUND_MS - 10 * 60_000);
    expect(year).toBe(latestExpiry(NOW).getTime());
    expect(checkNewKey(form({ expiry: '1y' }), NOW).ok).toBe(true);
  });

  it('LOG-007: a refused create gives a fixed sentence, never the hub text', () => {
    for (const code of ['CONFLICT', 'VALIDATION', 'FORBIDDEN', 'INTERNAL', 'UNAUTHENTICATED'] as const) {
      expect(createFailure({ code, message: 'hub text', fields: [] } as never)).not.toContain('hub text');
    }
    expect(createFailure({ code: 'CONFLICT', message: 'hub text' })).toBe('An API key with this name exists. Choose another name.');
  });
});

describe('the list (INT-003, UI-003)', () => {
  it('INT-003: a key whose expiry has passed is expired; one still ahead is not', () => {
    expect(isExpired({ expiresAt: '2026-09-01T00:00:00Z' }, NOW)).toBe(true);
    expect(isExpired({ expiresAt: '2027-06-01T00:00:00Z' }, NOW)).toBe(false);
  });

  it('INT-003: a key is expired from the exact instant of its expiry, and not one millisecond before', () => {
    const at = new Date('2027-06-01T00:00:00Z');
    expect(isExpired({ expiresAt: '2027-06-01T00:00:00Z' }, at)).toBe(true);
    expect(isExpired({ expiresAt: '2027-06-01T00:00:00Z' }, new Date(at.getTime() - 1))).toBe(false);
  });

  it('INT-003: scopes read Read and Write, and a key never used reads Never', () => {
    expect([scopeLabel('READ'), scopeLabel('WRITE')]).toEqual(['Read', 'Write']);
    expect(lastUsedText({ lastUsedAt: null })).toBe('Never');
  });
});
