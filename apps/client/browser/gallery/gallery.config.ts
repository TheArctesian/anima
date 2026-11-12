import { existsSync } from 'node:fs';
import { defineConfig, devices } from '@playwright/test';
import { runProxyUrl } from '../isolation/shared-proxy';
import { BUILD_FOLDER, BUILD_MISSING } from '../synthetic/static-files';

// The screenshot gallery: `bun run --cwd apps/client gallery`. Its files end in `.gallery.ts`, so `test:browsers` never runs them.
if (!existsSync(`${BUILD_FOLDER}index.html`)) throw new Error(`apps/client/build/ is missing: ${BUILD_MISSING}`);

export default defineConfig({
  globalTeardown: '../isolation/global-teardown.ts',
  testDir: '.',
  testMatch: '*.gallery.ts',
  outputDir: '../../node_modules/.playwright-gallery',
  timeout: 90_000,
  expect: { timeout: 15_000 },
  retries: 0,
  fullyParallel: true,
  workers: 3,
  reporter: 'list',
  use: { headless: true, baseURL: 'https://hub.anima.ing', reducedMotion: 'reduce' },
  // WebKit only (MV-001), behind the same deny-all proxy as the browser tests.
  projects: [{ name: 'webkit', use: { ...devices['Desktop Safari'], proxy: { server: await runProxyUrl('webkit') } } }],
});
