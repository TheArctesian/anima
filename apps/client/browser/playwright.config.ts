import { existsSync } from 'node:fs';
import { availableParallelism } from 'node:os';
import { defineConfig, devices } from '@playwright/test';
import { runProxyUrl } from './isolation/shared-proxy';
import { BUILD_FOLDER, BUILD_MISSING } from './synthetic/static-files';

// Not part of `bun run gates`: gate 7 runs before the build exists. Build first (`bun run --cwd apps/client build`), install
// the engines once (`bunx playwright install chromium webkit firefox`), then `bun run --cwd apps/client test:browsers`.
// Browser tests run against the production build, which the synthetic hub serves from `apps/client/build/` (WP-44 E-7).
// There is no web server and no network: every request ends in the hub inside the browser context.
if (!existsSync(`${BUILD_FOLDER}index.html`)) throw new Error(`apps/client/build/ is missing: ${BUILD_MISSING}`);

// The machines that run this are often loaded far past their core count (a load average of 40 to 90 on a 28-core host): the
// engines then start slowly, and many at once make each other slower still. So the default is a few workers, whatever the
// core count, and a test has room for a slow start. `ANIMA_BROWSER_WORKERS` sets the number for a quiet machine or a bad one.
const TEST_TIMEOUT_MS = 60_000;
const EXPECT_TIMEOUT_MS = 10_000;
const DEFAULT_WORKERS = Math.min(4, Math.max(1, Math.floor(availableParallelism() / 4)));
const workers = Number.parseInt(process.env.ANIMA_BROWSER_WORKERS ?? '', 10) || DEFAULT_WORKERS;

export default defineConfig({
  globalTeardown: './isolation/global-teardown.ts',
  testDir: '.',
  testMatch: '*.spec.ts',
  outputDir: '../node_modules/.playwright-results',
  timeout: TEST_TIMEOUT_MS,
  expect: { timeout: EXPECT_TIMEOUT_MS },
  retries: 0,
  fullyParallel: true,
  workers,
  reporter: 'list',
  use: { headless: true, baseURL: 'https://hub.anima.ing' },
  // Every engine goes through its own deny-all proxy (isolation/): no test browser can reach a real host (ruling S3).
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'], proxy: { server: await runProxyUrl('chromium') } } },
    { name: 'webkit', use: { ...devices['Desktop Safari'], proxy: { server: await runProxyUrl('webkit') } } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'], proxy: { server: await runProxyUrl('firefox') } } },
  ],
});
