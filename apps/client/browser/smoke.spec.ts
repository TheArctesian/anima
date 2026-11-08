import { expect, test } from './test-hub';
import { createHash } from 'node:crypto';
import { HUB_ORIGIN } from './synthetic/access';

// Smoke: the harness serves the production build, the client signs in by cookie and lists the fixture devices.

const FIXTURE_DEVICES = ['win-desktop', 'win-legacy', 'mac-mini', 'gpu-box', 'nas'];

test.describe('signed in', () => {
  test.use({ signedIn: true });

  test('MV-001: the device list shows the fixture devices, with the K7 headers and no other origin', async ({ page, hub, watch }) => {
    const response = await page.goto(`${HUB_ORIGIN}/devices`);
    for (const name of FIXTURE_DEVICES) await expect(page.getByText(name, { exact: true }).first()).toBeVisible();

    const headers = response?.headers() ?? {};
    expect(headers['content-security-policy']).toContain("script-src 'self' 'sha256-");
    expect(headers['content-security-policy']).toContain("connect-src 'self' wss://hub.anima.ing");
    expect(headers['x-content-type-options']).toBe('nosniff');
    expect(headers['referrer-policy']).toBe('no-referrer');
    expect(headers['cross-origin-opener-policy']).toBe('same-origin');
    expect(headers['strict-transport-security']).toBe('max-age=31536000');
    expect(headers['cache-control']).toBe('no-cache');

    // Every inline script of the built page is allowed by hash (a stale hash would block the app start).
    const inline = await page.locator('script:not([src])').evaluateAll((scripts) => scripts.map((script) => script.textContent ?? ''));
    expect(inline.length).toBeGreaterThan(0);
    for (const body of inline) {
      expect(headers['content-security-policy']).toContain(`'sha256-${createHash('sha256').update(body).digest('base64')}'`);
    }

    expect(hub.recording.foreignRequests, `foreign origins: ${hub.recording.foreignOrigins.join(', ')}`).toBe(0);
    expect(hub.recording.where({ operation: 'Devices' }).length).toBeGreaterThan(0);
    expect(watch.pageErrors).toEqual([]);
    expect(watch.consoleErrors).toEqual([]);
    // T5 owns the CSP: when the build violates the K7 policy, this fails and the line below is marked.
    expect(watch.cspViolations).toEqual([]);
  });
});
