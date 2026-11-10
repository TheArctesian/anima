// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CODE, settle, showPending, ENROLMENT, button, click, pass, setUp, show, tearDown, text } from './panel.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

beforeEach(setUp);
afterEach(tearDown);

describe('Install agent: focus, busy, copy and layout (WCAG 2.4.3, MET-019)', () => {
  const active = () => document.activeElement;

  async function focusAndClick(label: string): Promise<void> {
    button(label).focus();
    await click(label);
  }

  it('MET-019: after Install agent focus lands on the code, not on the page body', async () => {
    await show(ENROLMENT);
    await focusAndClick('Install agent');
    expect(active()?.textContent).toBe('Enrolment code');
  });

  it('MET-019: after Close focus returns to the Install agent button', async () => {
    await show(ENROLMENT);
    await focusAndClick('Install agent');
    await focusAndClick('Close');
    expect(active()).toBe(button('Install agent'));
  });

  it('MET-012: when the code expires while focus is in the panel, focus returns to the Install agent button', async () => {
    await show(ENROLMENT);
    await focusAndClick('Install agent');
    await pass(15 * 60);
    expect(active()).toBe(button('Install agent'));
  });

  it('MET-012: an expiry does not take focus from a control outside the panel', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    const elsewhere = document.createElement('input');
    document.body.append(elsewhere);
    elsewhere.focus();
    await pass(15 * 60);
    expect(active()).toBe(elsewhere);
  });
});

describe('Install agent: busy, copy and layout (MET-019)', () => {
  it('MET-019: while the code is being made the button says so', async () => {
    const answer = await showPending('enrolAgent', 'NONE');
    await click('Install agent');
    expect(button('Making a code…').getAttribute('aria-busy')).toBe('true');
    await answer({ ok: true, value: { code: CODE, expiresAt: '2026-11-09T10:15:00Z' } });
  });

  it('MET-019: the Copied status goes after a few seconds, and a second copy is announced again', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    await click('Copy command');
    const first = document.querySelector('[role="status"]');
    expect(first?.textContent).toBe('Copied.');
    await pass(2);
    await click('Copy command');
    expect(document.querySelector('[role="status"]')).not.toBe(first);
    await pass(5);
    expect(text()).not.toContain('Copied.');
  });

  it('MET-019: choosing another computer clears the Copied status of the old command', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    await click('Copy command');
    const picker = document.querySelector('select');
    if (!picker) throw new Error('no target picker');
    picker.value = 'macos-aarch64';
    picker.dispatchEvent(new Event('change', { bubbles: true }));
    await settle();
    expect(text()).not.toContain('Copied.');
  });

  it('MET-019: the computer picker is a labelled form field', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    const picker = document.querySelector('select');
    expect(picker?.labels?.[0]?.textContent).toBe('Computer');
  });

  it('MET-019: the timer is not a live region', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    expect(document.querySelector('[role="timer"]')?.getAttribute('aria-live')).toBe('off');
  });
});

