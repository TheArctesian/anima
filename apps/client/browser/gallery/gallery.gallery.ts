import { mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { test, type BrowserContext, type Page } from '@playwright/test';
import { installSyntheticHub } from '../synthetic/synthetic-hub';
import { SCREENS } from './screens';
import { HEIGHT, MODES, storedSettings, type Mode, type Screen } from './shots';

// One runner for every screen and every mode (`bun run --cwd apps/client gallery`). Pictures go to `apps/client/gallery-shots/`.

const SHOTS_FOLDER = fileURLToPath(new URL('../../gallery-shots/', import.meta.url));
const SETTLE_MS = 400;

const numbered = SCREENS.map((screen, index) => ({ screen, file: `${String(index + 1).padStart(2, '0')}-${screen.name}.png` }));

async function prepare(context: BrowserContext, chosen: Mode, screen: Screen): Promise<void> {
  const stored = storedSettings(chosen);
  await context.addInitScript((settings) => {
    for (const [key, value] of Object.entries(settings)) localStorage.setItem(key, value);
  }, stored);
  const hub = await installSyntheticHub(context, screen.fixtures === undefined ? {} : { fixtures: screen.fixtures });
  if (screen.signedOut !== true) await hub.signIn();
}

/** Fonts loaded, requests quiet and animations off, so two runs of one screen give the same picture. */
async function settle(page: Page): Promise<void> {
  await page.waitForLoadState('networkidle');
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(SETTLE_MS);
}

async function shoot(page: Page, screen: Screen, path: string): Promise<void> {
  if (screen.target !== undefined) await screen.target(page).screenshot({ path });
  else await page.screenshot({ path, fullPage: screen.windowOnly !== true });
}

for (const chosen of MODES) {
  test.describe(chosen.folder, () => {
    test.use({ viewport: { width: chosen.width, height: HEIGHT }, colorScheme: chosen.theme });
    mkdirSync(`${SHOTS_FOLDER}${chosen.folder}`, { recursive: true });
    for (const { screen, file } of numbered) {
      test(file, async ({ page, context }) => {
        await prepare(context, chosen, screen);
        await screen.open(page);
        await settle(page);
        await shoot(page, screen, `${SHOTS_FOLDER}${chosen.folder}/${file}`);
      });
    }
  });
}
