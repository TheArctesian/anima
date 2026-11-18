import { expect, test } from './test-hub';
import { HUB_ORIGIN } from './synthetic/access';

// The tab "Containers" of a device against the production build (HOST-001, HOST-013): the groups by Compose project, the
// log of the selected container, and a device the hub cannot read.

test.describe('signed in', () => {
  test.use({ signedIn: true });

  test('HOST-013: the tab lists the containers by Compose project and shows the log of the selected one', async ({ page, hub, watch }) => {
    await page.goto(`${HUB_ORIGIN}/devices`);
    await page.getByText('nas', { exact: true }).first().click();
    await page.getByRole('tab', { name: 'Containers' }).click();

    await expect(page.getByRole('rowheader', { name: /^tracks\s*2 of 3 running$/ })).toBeVisible();
    await expect(page.getByRole('rowheader', { name: /^No Compose project/ })).toBeVisible();
    await expect(page.getByRole('row', { name: /tracks-worker-1/ })).toContainText('Exited (1) 2 hours ago');
    expect(hub.recording.where({ operation: 'ContainerLog' })).toHaveLength(0);

    await page.getByRole('button', { name: 'Log of tracks-app-1' }).click();
    await expect(page.getByRole('heading', { name: 'Log of tracks-app-1' })).toBeVisible();
    await expect(page.getByRole('list', { name: 'Log lines, oldest first' }).getByRole('listitem')).toHaveCount(12);
    expect(hub.recording.where({ operation: 'ContainerLog' })).toHaveLength(1);

    const shot = process.env.ANIMA_CONTAINERS_SHOT;
    if (shot !== undefined) await page.screenshot({ path: shot, fullPage: true });
    expect(watch.pageErrors).toEqual([]);
    expect(watch.consoleErrors).toEqual([]);
    expect(watch.cspViolations).toEqual([]);
  });

  test('HOST-001: a device with no docker command shows the reason of the hub', async ({ page, watch }) => {
    await page.goto(`${HUB_ORIGIN}/devices`);
    await page.getByText('mac-mini', { exact: true }).first().click();
    await page.getByRole('tab', { name: 'Containers' }).click();
    await expect(page.getByRole('alert')).toContainText('finds no docker command');
    await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
    expect(watch.pageErrors).toEqual([]);
  });
});
