import { expect, test } from '../test-hub';
import { HUB_ORIGIN } from '../synthetic/access';
import { installSyntheticHub } from '../synthetic/synthetic-hub';

// The Agent usage page over the synthetic hub: the fixture Mac with Claude Code and Codex, a system unit, a device turned off.

test.describe('signed in', () => {
  test.use({ signedIn: true });

  test('AIS-020: the page is in the navigation and shows the summary, the table and the notes', async ({ page, watch }) => {
    await page.goto(`${HUB_ORIGIN}/devices`);
    await page.getByRole('link', { name: 'Agent usage' }).click();
    await expect(page).toHaveURL(`${HUB_ORIGIN}/agents/usage`);
    await expect(page.getByRole('heading', { name: 'Agent usage', level: 1 })).toBeVisible();
    await expect(page.locator('.summary')).toContainText('sessions');
    await expect(page.getByRole('table')).toContainText('mac-mini');
    await expect(page.getByRole('table')).toContainText('No price');
    await expect(page.getByText('It is not the bill of a subscription.')).toBeVisible();
    await expect(page.getByText('This daemon runs as a system unit and cannot see the sessions of a user.')).toBeVisible();
    await expect(page.getByText('The use counts are turned off in the settings of the daemon on this device.')).toBeVisible();
    expect(watch.pageErrors).toEqual([]);
  });

  test('AIS-020: a choice is in the address, and an address restores it', async ({ page }) => {
    await page.goto(`${HUB_ORIGIN}/agents/usage?days=90&by=MODEL&source=CODEX`);
    await expect(page.getByRole('radio', { name: '90 days' })).toBeChecked();
    await expect(page.getByRole('columnheader', { name: 'Model' })).toBeVisible();
    await expect(page.getByRole('columnheader', { name: 'Device' })).toHaveCount(0);
    await expect(page.getByRole('table')).toContainText('gpt-5-codex');
    await expect(page.getByRole('table')).not.toContainText('claude-opus-4');
    await page.getByRole('radio', { name: '7 days' }).click();
    await expect(page).toHaveURL(/days=7/);
  });
});

test('AIS-020: with no device the page is an empty state with Install daemon', async ({ browser }) => {
  const context = await browser.newContext();
  const hub = await installSyntheticHub(context, { fixtures: (loaded) => ({ ...loaded, devices: [] }) });
  await hub.signIn();
  const page = await context.newPage();
  await page.goto(`${HUB_ORIGIN}/agents/usage`);
  await expect(page.getByText('No use counts yet')).toBeVisible();
  await expect(page.getByRole('link', { name: 'Install daemon' })).toBeVisible();
  await context.close();
});
