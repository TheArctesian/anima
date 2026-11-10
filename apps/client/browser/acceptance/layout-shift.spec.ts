import type { Page } from '@playwright/test';
import { expect, test } from '../test-hub';
import { DEVICE_IDS } from './screens';

// R-096: a page whose content arrives late must not push what is already shown. Layout shift is measured by the engine's own
// `layout-shift` entries, which only Chromium reports, under a hub that answers GraphQL 400 ms late.

const API_DELAY_MS = 400;
const SETTLE_MS = 1500;
const WIDTH = 1280;
const FIRST_PASS_HEIGHT = 720;
const CLS_LIMIT = 0.01;

/**
 * One line per page; a page added later is one more entry. `enabled` names a field that must be usable too, for a page with
 * several late sections. `tall` gives the page a viewport exactly as high as the page (measured by a first pass), since the
 * engine reports only shifts inside the viewport and a long page shifts below the fold; a taller viewport would dilute the
 * score, because a shift counts its area against the viewport's.
 */
const PAGES = [
  { name: 'the device list', path: '/devices', shown: { role: 'link', name: 'mac-studio' } },
  { name: 'a device detail page', path: `/devices/${DEVICE_IDS.gpuBox}`, shown: { role: 'heading', name: 'gpu-box' } },
  { name: 'the settings page', path: '/settings', shown: { role: 'button', name: 'Revoke metrics-scrape' }, enabled: 'Interval (seconds)', tall: true },
] as const;

type Shown = (typeof PAGES)[number]['shown'];

const SHIFT = 'animaLayoutShift';

test.skip(({ browserName }) => browserName !== 'chromium', 'only Chromium reports layout-shift entries');
test.use({ signedIn: true });

async function watchLayoutShift(page: Page): Promise<void> {
  await page.addInitScript((key) => {
    Reflect.set(window, key, 0);
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        const shift = entry as PerformanceEntry & { hadRecentInput: boolean; value: number };
        if (!shift.hadRecentInput) Reflect.set(window, key, (Reflect.get(window, key) as number) + shift.value);
      }
    }).observe({ type: 'layout-shift', buffered: true });
  }, SHIFT);
}

async function slowGraphql(page: Page): Promise<void> {
  await page.route(
    (url) => url.pathname === '/graphql',
    async (route) => {
      if (route.request().method() === 'POST') await new Promise((done) => setTimeout(done, API_DELAY_MS));
      await route.fallback();
    },
  );
}

async function settle(page: Page, entry: { shown: Shown; enabled?: string }): Promise<void> {
  const { shown, enabled } = entry;
  await expect(page.getByRole(shown.role, { name: shown.name, exact: true })).toBeVisible();
  if (enabled !== undefined) await expect(page.getByLabel(enabled, { exact: true })).toBeEnabled();
  await page.waitForTimeout(SETTLE_MS);
}

/** Where every section heading sits, top edge in page pixels, by its text. */
async function headingTops(page: Page): Promise<Record<string, number>> {
  return page.evaluate(() => Object.fromEntries([...document.querySelectorAll('h2')].map((h) => [h.textContent, Math.round(h.getBoundingClientRect().top + scrollY)])));
}

async function layoutShiftOf(page: Page, entry: { path: string; shown: Shown; enabled?: string; tall: boolean }): Promise<{ shift: number; moved: string[] }> {
  if (entry.tall) {
    // First pass: how high the settled page is. The viewport then is that high, and the page loads again from nothing.
    await page.setViewportSize({ width: WIDTH, height: FIRST_PASS_HEIGHT });
    await slowGraphql(page);
    await page.goto(entry.path);
    await settle(page, entry);
    const height = await page.evaluate(() => document.documentElement.scrollHeight);
    await page.setViewportSize({ width: WIDTH, height });
  } else {
    await slowGraphql(page);
  }
  await watchLayoutShift(page);
  await page.goto(entry.path);
  const first = await headingTops(page);
  await settle(page, entry);
  const last = await headingTops(page);
  const moved = Object.keys(first).filter((heading) => first[heading] !== last[heading]);
  return { shift: await page.evaluate((key) => Reflect.get(window, key) as number, SHIFT), moved };
}

for (const entry of PAGES) {
  const { name, path, shown } = entry;
  const enabled = 'enabled' in entry ? entry.enabled : undefined;
  const tall = 'tall' in entry && entry.tall;
  test(`R-096: ${name} shifts less than ${String(CLS_LIMIT)} while its data arrives late`, async ({ page }) => {
    const { shift, moved } = await layoutShiftOf(page, { path, shown, enabled, tall });
    expect(shift).toBeLessThanOrEqual(CLS_LIMIT);
    // A section heading is already where it will stay when the first paint is done.
    expect(moved).toEqual([]);
  });
}

test('R-096: the check sees a block that arrives late and pushes content', async ({ page }) => {
  await watchLayoutShift(page);
  await page.goto('/devices');
  await expect(page.getByRole('link', { name: 'mac-studio', exact: true })).toBeVisible();
  await page.evaluate(() => {
    const block = document.createElement('div');
    block.style.cssText = 'block-size: 200px';
    block.textContent = 'late block';
    document.body.prepend(block);
  });
  await page.waitForTimeout(SETTLE_MS);
  expect(await page.evaluate((key) => Reflect.get(window, key) as number, SHIFT)).toBeGreaterThan(CLS_LIMIT);
});
