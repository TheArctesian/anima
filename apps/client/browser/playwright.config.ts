import { existsSync } from 'node:fs';
import { defineConfig, devices } from '@playwright/test';
import { BUILD_FOLDER, BUILD_MISSING } from './synthetic/static-files';

// Browser tests run against the production build, which the synthetic hub serves from `apps/client/build/` (WP-44 E-7).
// There is no web server and no network: every request ends in the hub inside the browser context.
if (!existsSync(`${BUILD_FOLDER}index.html`)) throw new Error(`apps/client/build/ is missing: ${BUILD_MISSING}`);

const TEST_TIMEOUT_MS = 30_000;
const EXPECT_TIMEOUT_MS = 10_000;

export default defineConfig({
  testDir: '.',
  testMatch: '*.spec.ts',
  outputDir: '../node_modules/.playwright-results',
  timeout: TEST_TIMEOUT_MS,
  expect: { timeout: EXPECT_TIMEOUT_MS },
  retries: 0,
  fullyParallel: true,
  reporter: 'list',
  use: { headless: true, baseURL: 'https://hub.anima.ing' },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
  ],
});
