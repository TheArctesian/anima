import { describe, expect, it } from 'vitest';
import { FIXTURE_SETTINGS } from '../notifications.fixture';
import { BODY_MISSING, D21_PLACEHOLDERS, templateFine, templateProblems, TITLE_MISSING } from './template';

const allowed = FIXTURE_SETTINGS.placeholders;

describe('template placeholders', () => {
  it('D-21: the fallback list is the D-21 list the K6 settings carry', () => {
    expect(D21_PLACEHOLDERS).toEqual(allowed);
  });

  it('NOT-007, D-21: a template that uses only the placeholders passes', () => {
    expect(templateProblems(FIXTURE_SETTINGS.template, allowed)).toEqual({ title: '', body: '' });
    expect(templateFine(templateProblems({ title: 'Plain title', body: '{days} days left on {check}' }, allowed))).toBe(true);
  });

  it('D-21: a placeholder outside the list is refused and named', () => {
    const { title, body } = templateProblems({ title: '{device} on {host}', body: '{device} {host} {host} {Device} { device } {}' }, allowed);
    expect(title).toBe(`Not a placeholder: {host}. Use only ${allowed.join(' ')}.`);
    expect(body).toBe(`Not a placeholder: {host}, {Device}, { device } and others. Use only ${allowed.join(' ')}.`);
    expect(templateFine(templateProblems({ title: '{device}', body: '{secret}' }, allowed))).toBe(false);
  });

  it('D-21: the message names at most three unknown placeholders, each cut short', () => {
    const long = `{${'x'.repeat(100)}}`;
    const problem = templateProblems({ title: 'Plain title', body: `{a} {b} ${long} {d}` }, allowed).body;
    expect(problem).toMatch(/^Not a placeholder: \{a\}, \{b\}, \{x+… and others\./);
    expect(problem).not.toContain('x'.repeat(40));
  });

  it('NOT-007: an empty title or body is refused', () => {
    expect(templateProblems({ title: '  ', body: '' }, allowed)).toEqual({ title: TITLE_MISSING, body: BODY_MISSING });
  });
});
