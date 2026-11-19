import { expect, test } from '../test-hub';
import { HUB_ORIGIN } from '../synthetic/access';

// The tab "Containers" of a device against the production build (HOST-001, HOST-013, HOST-015, HOST-017 to HOST-021): the
// groups by Compose project with the state of each, the log of the selected container with its tools, the details, the
// mark of a newer image, the images, volumes and networks, and a device the hub cannot read.

test.describe('signed in', () => {
  test.use({ signedIn: true });

  test('HOST-013: the tab lists the containers by Compose project and shows the log of the selected one', async ({ page, hub, watch }) => {
    await page.goto(`${HUB_ORIGIN}/devices`);
    await page.getByText('nas', { exact: true }).first().click();
    await page.getByRole('tab', { name: 'Containers' }).click();

    await expect(page.getByRole('rowheader', { name: /^tracks\s*Partly running\s*2 of 3 running$/ })).toBeVisible();
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

test.describe('signed in, more to read of Docker', () => {
  test.use({ signedIn: true });

  test('HOST-015 HOST-017 HOST-021: the log tools, the details of a container and the mark of a newer image', async ({ page, hub, watch }) => {
    await page.goto(`${HUB_ORIGIN}/devices`);
    await page.getByText('nas', { exact: true }).first().click();
    await page.getByRole('tab', { name: 'Containers' }).click();

    await page.getByRole('button', { name: 'Log of tracks-app-1' }).click();
    const lines = page.getByRole('list', { name: 'Log lines, oldest first' }).getByRole('listitem');
    await expect(lines).toHaveCount(12);
    await page.getByRole('radio', { name: 'Warning' }).click();
    await expect(lines).toHaveCount(3);
    await page.getByRole('searchbox', { name: 'Search the log' }).fill('request 8 ');
    await expect(lines).toHaveCount(1);
    await expect(page.getByText('1 of 12 lines')).toBeVisible();
    const download = page.waitForEvent('download');
    await page.getByRole('button', { name: 'Download' }).click();
    expect((await download).suggestedFilename()).toBe('tracks-app-1.log');

    expect(hub.recording.where({ operation: 'ContainerDetail' })).toHaveLength(0);
    await page.getByRole('button', { name: 'Details of tracks-app-1' }).click();
    const details = page.getByRole('region', { name: 'Details of tracks-app-1' });
    await expect(details.getByRole('region', { name: 'Environment' })).toContainText('SECRET_KEY_BASE');
    await expect(details.getByRole('region', { name: 'Environment' })).toContainText('Anima never reads a value');
    await expect(details.getByRole('region', { name: 'Mounts' })).toContainText('volume tracks-data → /data (read-write)');
    await expect(page.getByRole('heading', { name: 'Log of tracks-app-1' })).toHaveCount(0);

    expect(hub.recording.where({ operation: 'ContainerImageUpdates' })).toHaveLength(0);
    await page.getByRole('button', { name: 'Look for newer images' }).click();
    await expect(page.getByRole('row', { name: /tracks-app-1/ })).toContainText('Newer image');
    await expect(page.getByRole('row', { name: /tracks-db-1/ })).toContainText('Up to date');
    await expect(page.getByText('4 containers, 1 with a newer image')).toBeVisible();

    const shot = process.env.ANIMA_CONTAINERS_DETAIL_SHOT;
    if (shot !== undefined) await page.screenshot({ path: shot, fullPage: true });
    expect(watch.pageErrors).toEqual([]);
    expect(watch.consoleErrors).toEqual([]);
    expect(watch.cspViolations).toEqual([]);
  });

  test('HOST-020: the images, the volumes and the networks of a device, each read when it is first opened', async ({ page, hub, watch }) => {
    await page.goto(`${HUB_ORIGIN}/devices`);
    await page.getByText('nas', { exact: true }).first().click();
    await page.getByRole('tab', { name: 'Containers' }).click();
    await expect(page.getByRole('row', { name: /recorder/ })).toBeVisible();
    expect(hub.recording.where({ operation: 'ContainerStorage' })).toHaveLength(0);

    await page.getByRole('radio', { name: 'Images' }).click();
    await expect(page.getByRole('row', { name: /example\/tracks:0\.36/ })).toContainText('tracks-app-1, tracks-worker-1');
    await expect(page.getByText('3 images. 921.6MB in use, 13.6MB (1%) can be freed by a clean-up')).toBeVisible();
    await page.getByRole('radio', { name: 'Volumes' }).click();
    await expect(page.getByRole('row', { name: /old-data/ })).toContainText('Not used');
    expect(hub.recording.where({ operation: 'ContainerStorage' })).toHaveLength(1);
    await page.getByRole('radio', { name: 'Networks' }).click();
    await expect(page.getByRole('row', { name: /tracks_default/ })).toContainText('tracks-app-1, tracks-db-1, tracks-worker-1');

    const shot = process.env.ANIMA_CONTAINERS_NETWORKS_SHOT;
    if (shot !== undefined) await page.screenshot({ path: shot, fullPage: true });
    expect(watch.pageErrors).toEqual([]);
    expect(watch.consoleErrors).toEqual([]);
    expect(watch.cspViolations).toEqual([]);
  });
});
