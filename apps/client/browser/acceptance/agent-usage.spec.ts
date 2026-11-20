import { expect, test } from '../test-hub';
import { HUB_ORIGIN } from '../synthetic/access';
import { installSyntheticHub } from '../synthetic/synthetic-hub';

// The Agent usage page over the synthetic hub: the fixture Mac with Claude Code and Codex, a system unit, a device turned off.

test.describe('signed in', () => {
  test.use({ signedIn: true });

  test('AIS-020: the page is in the navigation and shows the repositories, the totals, the days and the two tables', async ({ page, watch }) => {
    await page.goto(`${HUB_ORIGIN}/devices`);
    await page.getByRole('link', { name: 'Agent usage' }).click();
    await expect(page).toHaveURL(`${HUB_ORIGIN}/agents/usage`);
    await expect(page.getByRole('heading', { name: 'Agent usage', level: 1 })).toBeVisible();
    await expect(page.getByRole('navigation', { name: 'Repositories' })).toContainText('example/anima');
    await expect(page.getByRole('region', { name: 'Totals' })).toContainText('not a bill');
    await expect(page.getByRole('list', { name: 'Estimated cost for each day (UTC)' }).getByRole('listitem')).toHaveCount(30);
    await expect(page.getByRole('table', { name: 'By program and model' })).toContainText('No price');
    const devices = page.getByRole('table', { name: 'By device' });
    await expect(devices).toContainText('mac-mini');
    await expect(devices).toContainText("Runs as a system unit: it cannot see a user's sessions");
    await expect(devices).toContainText("Turned off in the daemon's settings");
    expect(watch.pageErrors).toEqual([]);
  });

  test('AIS-020: a choice is in the address, and an address restores it', async ({ page }) => {
    await page.goto(`${HUB_ORIGIN}/agents/usage?days=90&count=sessions&source=CODEX`);
    await expect(page.getByRole('radio', { name: '90 days' })).toBeChecked();
    await expect(page.getByRole('radio', { name: 'Sessions' })).toBeChecked();
    const models = page.getByRole('table', { name: 'By program and model' });
    await expect(models).toContainText('gpt-5-codex');
    await expect(models).not.toContainText('claude-opus-4');
    await page.getByRole('radio', { name: '7 days' }).click();
    await expect(page).toHaveURL(/days=7/);
  });

  test('a repository of the list limits the page to it, and the list stays whole', async ({ page }) => {
    await page.goto(`${HUB_ORIGIN}/agents/usage`);
    const repositories = page.getByRole('navigation', { name: 'Repositories' });
    await repositories.getByRole('button', { name: /example\/site/ }).click();
    await expect(page).toHaveURL(/repository=github\.com%2Fexample%2Fsite/);
    await expect(page.getByRole('table', { name: 'By program and model' })).not.toContainText('claude-opus-4');
    await expect(repositories).toContainText('example/anima');
  });
});

test('AIS-020: with no device the page is an empty state that leads to the Devices page', async ({ browser }) => {
  const context = await browser.newContext();
  const hub = await installSyntheticHub(context, { fixtures: (loaded) => ({ ...loaded, devices: [] }) });
  await hub.signIn();
  const page = await context.newPage();
  await page.goto(`${HUB_ORIGIN}/agents/usage`);
  await expect(page.getByText('No use counts yet')).toBeVisible();
  await expect(page.getByText('To install it, open a device, then Settings, then Daemon.')).toBeVisible();
  await expect(page.getByRole('link', { name: 'Open Devices' })).toBeVisible();
  await context.close();
});
