import type { Page } from '@playwright/test';
import { expect, test } from '../test-hub';
import { signInWith } from '../sign-in-steps';
import { HUB_ORIGIN } from '../synthetic/access';
import { k7Policy } from './k7-policy';
import { expectNoViolations, servePolicy, violationsSoFar, type PolicyReports } from './violations';

// The production build under the exact K7 policy, with the script hashes the hub computes from index.html (WP-43 A-2):
// each screen boots and works with no `securitypolicyviolation` in the page and no CSP report from the browser (E-6).

const MAC_MINI = '0190a000-0000-7000-8012-000000000003';
const DETAIL_TABS = ['Checks', 'Sessions', 'Logs', 'Settings', 'Overview'];

test('SEC-001: the synthetic hub sends the K7 text with the A-2 hashes and the public WebSocket origin', async ({ page }) => {
  // Every other browser spec runs under this header, so it must be the contract's text, byte for byte.
  const response = await page.goto(`${HUB_ORIGIN}/sign-in`);
  const header = response?.headers()['content-security-policy'];
  expect(header).toBe(k7Policy());
  expect(header).toContain("connect-src 'self' wss://hub.anima.ing;");
  expect(header).toMatch(/script-src 'self' 'sha256-[A-Za-z0-9+/]{43}=';/);
});

let policy: PolicyReports;

async function openDevices(page: Page): Promise<void> {
  await page.goto(`${HUB_ORIGIN}/devices`);
  await expect(page.getByText('mac-mini', { exact: true }).first()).toBeVisible();
}

/** Serves each test of the enclosing group under K7 with the report channel, and re-checks both channels after it. */
function eachTestUnderK7(): void {
  test.beforeEach(async ({ context }) => {
    policy = await servePolicy(context, k7Policy());
  });
  // A violation after the explicit check, while the page keeps running, still fails the test.
  test.afterEach(({ watch }) => {
    expect(violationsSoFar(watch, policy)).toEqual({ events: [], reports: [] });
  });
}

test.describe('signed out, under K7 with a report channel', () => {
  eachTestUnderK7();

  test('SEC-001: the sign-in page boots and sends a sign-in under K7 with no violation', async ({ page, hub, watch }) => {
    await page.goto(`${HUB_ORIGIN}/sign-in`);
    await signInWith(page, 'not-the-password');
    await expect.poll(() => hub.recording.where({ channel: 'graphql' }).length).toBeGreaterThan(0);
    await expectNoViolations(page, watch, policy);
  });
});

test.describe('signed in, under K7 with a report channel', () => {
  eachTestUnderK7();
  test.use({ signedIn: true });

  test('SEC-001: the device list and the live sessions panel run under K7 with no violation', async ({ page, watch }) => {
    await openDevices(page);
    await page.getByRole('button', { name: /^Live sessions/ }).click();
    await expect(page.getByRole('button', { name: /^Live sessions/ })).toHaveAttribute('aria-expanded', 'true');
    await expectNoViolations(page, watch, policy);
  });

  test('SEC-001: a device detail page and each of its tabs run under K7 with no violation', async ({ page, watch }) => {
    await page.goto(`${HUB_ORIGIN}/devices/${MAC_MINI}`);
    await expect(page.getByRole('heading', { level: 1, name: 'mac-mini' })).toBeVisible();
    for (const name of DETAIL_TABS) {
      await page.getByRole('tab', { name }).click();
      await expect(page.getByRole('tabpanel', { name })).toBeVisible();
    }
    await expectNoViolations(page, watch, policy);
  });

  test('SEC-001: a VNC session opens its tunnel and draws under K7 with no violation', async ({ page, hub, watch }) => {
    await openDevices(page);
    await page.getByRole('button', { name: 'VNC to mac-mini' }).first().click();
    await expect
      .poll(
        () =>
          hub.recording.where({
            channel: 'tunnel',
            operation: 'ticket accepted',
          }).length,
      )
      .toBe(1);
    await expect(page.getByRole('tab', { name: /mac-mini/ })).toBeVisible();
    await expect(page.locator('canvas').first()).toBeVisible();
    await expectNoViolations(page, watch, policy);
  });

  test('SEC-001: settings, with the theme cross-fade, Reduce motion and API keys, run under K7 with no violation', async ({ page, watch }) => {
    await page.goto(`${HUB_ORIGIN}/settings`);
    await expect(page.getByRole('heading', { level: 1, name: 'Settings' })).toBeVisible();
    await expect(page.getByRole('heading', { level: 2, name: 'API keys' })).toBeVisible();
    // Each theme change runs the view-transition cross-fade (WP-50); each motion choice rewrites the root's attributes.
    for (const theme of ['light', 'dark']) {
      await page.getByLabel('Theme').selectOption(theme);
      await expect(page.getByLabel('Theme')).toHaveValue(theme);
    }
    for (const motion of ['reduce', 'full']) {
      await page.getByLabel('Reduce motion').selectOption(motion);
      await expect(page.getByLabel('Reduce motion')).toHaveValue(motion);
    }
    await expectNoViolations(page, watch, policy);
  });
});
