import { expect, test } from './test-hub';
import { HUB_ORIGIN } from './synthetic/access';

// The web client as the hub serves it (K7 "HTTP surface"): fallback, 404 under /_app/, cache headers, content types.

test('K7: an unknown path gets index.html, a missing /_app/ file gets 404, both with the CSP', async ({ page }) => {
  const route = await page.goto(`${HUB_ORIGIN}/no/such/page`);
  expect(route?.status()).toBe(200);
  expect(route?.headers()['content-type']).toBe('text/html; charset=utf-8');
  expect(route?.headers()['content-security-policy']).toContain("default-src 'self'");

  const missing = await page.goto(`${HUB_ORIGIN}/_app/immutable/missing.js`);
  expect(missing?.status()).toBe(404);
  expect(missing?.headers()['content-security-policy']).toContain("default-src 'self'");
  expect(missing?.headers()['cache-control']).toBe('no-cache');
});

test('K7: files under /_app/immutable/ are immutable with their content type; the page is no-cache', async ({ page }) => {
  const scripts: { type: string; cache: string }[] = [];
  page.on('response', (response) => {
    const headers = response.headers();
    if (new URL(response.url()).pathname.startsWith('/_app/immutable/') && response.url().endsWith('.js')) {
      scripts.push({ type: headers['content-type'] ?? '', cache: headers['cache-control'] ?? '' });
    }
  });
  await page.goto(`${HUB_ORIGIN}/sign-in`);
  await expect.poll(() => scripts.length).toBeGreaterThan(0);
  for (const script of scripts) {
    expect(script).toEqual({ type: 'text/javascript; charset=utf-8', cache: 'public, max-age=31536000, immutable' });
  }
});
