// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import PlanUsageCard from './PlanUsageCard.svelte';
import { ESTIMATE_NOTE, NOT_AVAILABLE, NOT_CHARGED, OWES, SIGNED_OUT, parseUsage, type UsageAnswer } from './usage';
import { POOL_ANSWER, withHub } from './usage.fixture';

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const NOW = new Date('2026-11-20T08:05:00Z');
const words = () => (document.querySelector('section')?.textContent ?? '').replace(/\s+/g, ' ');

async function card(answer: UsageAnswer, dataBytes: number | null = 1_200_000_000): Promise<void> {
  app = mount(PlanUsageCard, { target: document.body, props: { read: () => Promise.resolve(answer), dataBytes, now: () => NOW } });
  await tick();
  await tick();
  flushSync();
}

const figures = (answer: unknown = POOL_ANSWER): UsageAnswer => {
  const usage = parseUsage(answer);
  if (usage === undefined) throw new Error('the fixture must parse');
  return { state: 'figures', usage };
};

describe('the plan and its use on the page of the hub (HMD-073 in form B)', () => {
  it('shows the data of the hub and the disk of the plan as two figures, the hours, the amount with its words and the time', async () => {
    await card(figures());
    const shown = words();
    expect(shown).toContain('Data of your hub');
    expect(shown).toContain('Disk used of the plan');
    expect(shown).toMatch(/\(25%\)/);
    expect(shown).toContain('12 h');
    expect(shown).toContain('$4.12');
    expect(shown).toContain(ESTIMATE_NOTE);
    expect(shown).toContain('Figures from 5 min ago');
    expect(shown).not.toContain(OWES);
  });

  it('on a plan that is not charged shows the hours and no amount', async () => {
    await card(figures(withHub({ plan: 'free' })));
    expect(words()).toContain(NOT_CHARGED);
    expect(words()).not.toContain(ESTIMATE_NOTE);
    expect(words()).toContain('0.5 h');
  });

  it('on a charged plan with no prices shows no amount and no estimate', async () => {
    await card(figures({ ...POOL_ANSWER, prices_set: false }));
    expect(words()).toMatch(new RegExp(`This month ?${NOT_AVAILABLE}`));
    expect(words()).not.toContain(ESTIMATE_NOTE);
  });

  it('for an account that owes (a member of the site) points to the account page and shows no sum of it', async () => {
    await card(figures({ ...POOL_ANSWER, version: 1, owes: true }));
    expect(words()).toContain(OWES);
    expect(document.querySelector('a')?.getAttribute('href')).toBe('https://cloud.anima.ing/account');
  });

  it('says why there are no figures, and keeps the data of the hub', async () => {
    await card({ state: 'unavailable' });
    expect(words()).toContain(NOT_AVAILABLE);
    expect(words()).toContain('Data of your hub');
    if (app) await unmount(app);
    app = undefined;
    document.body.replaceChildren();
    await card({ state: 'signed-out' }, null);
    expect(words()).toContain(SIGNED_OUT);
    expect(words()).toContain('Not measured yet');
  });
});
