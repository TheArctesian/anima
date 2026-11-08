import { expect, test } from '../test-hub';
import { HUB_ORIGIN } from '../synthetic/access';
import { DEVICE_IDS, openDevices } from './screens';
import { updaterChunkPaths } from './updater-files';

// Plan criterion 2, client side, against the synthetic hub: the web client lists devices, shows one, follows live status
// and the logs, and offers none of the desktop-only controls. Sessions have their own spec.

const DEVICE_NAMES = ['win-desktop', 'win-legacy', 'mac-mini', 'gpu-box', 'nas', 'build-box', 'amd-box', 'mac-studio'];
const AT = '2026-11-06T10:30:00Z';

test.use({ signedIn: true });

test.describe('what works', () => {
  test('CLI-001: the device list shows every device with its status', async ({ page }) => {
    await openDevices(page);
    for (const name of DEVICE_NAMES) await expect(page.getByRole('link', { name, exact: true })).toBeVisible();
    await expect(page.getByText('Up', { exact: true })).toHaveCount(DEVICE_NAMES.length);
  });

  test('CLI-001: a device opens in the detail view with its tabs, its checks and its SSH host key', async ({ page }) => {
    await openDevices(page);
    await page.getByRole('link', { name: 'gpu-box', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'gpu-box' })).toBeVisible();
    await expect(page).toHaveURL(`${HUB_ORIGIN}/devices/${DEVICE_IDS.gpuBox}`);
    for (const name of ['Overview', 'Checks', 'Sessions', 'Logs', 'Settings']) await expect(page.getByRole('tab', { name })).toBeVisible();
    await expect(page.getByText('ssh-ed25519', { exact: false })).toBeVisible();
    await page.getByRole('tab', { name: 'Checks' }).click();
    await expect(page.getByRole('tabpanel', { name: 'Checks' })).toBeVisible();
  });

  test('CLI-001: a status the hub pushes over the socket shows on the device at once, without a reload', async ({ page, hub }) => {
    await openDevices(page);
    const macMini = page.locator('article, li, tr', { has: page.getByRole('link', { name: 'mac-mini', exact: true }) }).first();
    await expect(macMini.getByText('Up', { exact: true })).toBeVisible();
    await expect.poll(() => hub.recording.where({ channel: 'graphql-socket', operation: 'DeviceStatusChanged' }).length).toBe(1);

    expect(hub.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: DEVICE_IDS.macMini, status: 'DOWN', at: AT } })).toBe(1);
    await expect(macMini.getByText('Down', { exact: true })).toBeVisible();
    await expect(macMini.getByText('Up', { exact: true })).toHaveCount(0);
    // Only that device changed.
    await expect(page.getByText('Down', { exact: true })).toHaveCount(1);
    await expect(page.getByText('Connected', { exact: true })).toBeVisible();
  });

  test('LOG-008: the logs panel lists the hub events and adds one the hub pushes', async ({ page, hub }) => {
    await openDevices(page);
    const logs = page.getByRole('region', { name: 'Logs' });
    await expect(logs.getByText('Check recovered', { exact: true })).toBeVisible();
    await expect(logs.getByText('Sign-in refused', { exact: true })).toBeVisible();
    await expect.poll(() => hub.recording.where({ channel: 'graphql-socket', operation: 'EventAdded' }).length).toBe(1);

    const event = {
      id: '0190a000-0000-7000-8020-0000000000ff',
      // The panel shows the last 24 hours by default, by the browser's clock.
      at: new Date().toISOString(),
      severity: 'ERROR',
      source: 'CHECK',
      deviceId: DEVICE_IDS.nas,
      deviceName: 'nas',
      message: 'Check went down',
    };
    expect(hub.push('EventAdded', { eventAdded: event })).toBe(1);
    await expect(logs.getByText('Check went down', { exact: true })).toBeVisible();
  });
});

test.describe('what the web client leaves to the desktop app', () => {
  test('CLI-001, UPD-001, LOG-008: Settings has no Updates row and no log folder button', async ({ page }) => {
    await page.goto(`${HUB_ORIGIN}/settings`, { waitUntil: 'domcontentloaded' });
    await expect(page.getByRole('heading', { level: 2, name: 'Appearance' })).toBeVisible();
    await expect(page.getByRole('heading', { level: 2, name: 'API keys' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Updates' })).toHaveCount(0);
    await expect(page.getByRole('button', { name: /update/i })).toHaveCount(0);
    await expect(page.getByRole('button', { name: /log folder/i })).toHaveCount(0);
    await expect(page.getByText(/update/i)).toHaveCount(0);
  });

  test('CLI-001: Add device offers no "This Mac" or "This computer" choice and no System Settings button', async ({ page }) => {
    await openDevices(page);
    await page.getByRole('button', { name: 'Add device' }).click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole('radio', { name: /^This (Mac|computer)/ })).toHaveCount(0);
    await expect(dialog.getByText(/This (Mac|computer)/)).toHaveCount(0);
    await expect(dialog.getByRole('button', { name: /Remote Login|System Settings/ })).toHaveCount(0);
  });

  test('CLI-001: Set up screen sharing starts no System Settings step in the page', async ({ page }) => {
    await openDevices(page);
    await expect(page.getByRole('button', { name: /System Settings/ })).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Set up screen sharing for nas' })).toBeVisible();
  });
});

test.describe('the updater', () => {
  test('UPD-001: no updater chunk is requested and nothing calls Tauri `invoke` on the list, a device, or Settings', async ({ page, context }) => {
    const updater = updaterChunkPaths();
    expect(updater.length, 'the build holds the updater chunk the page must not load').toBeGreaterThan(0);
    const requested: string[] = [];
    context.on('request', (request) => requested.push(new URL(request.url()).pathname));
    await context.addInitScript(() => {
      const calls: string[] = [];
      Reflect.set(window, 'animaTauriCalls', calls);
      // Not `window.isTauri`: this is a browser, but a call that reached for the shell would land here.
      Reflect.set(window, '__TAURI_INTERNALS__', { invoke: (command: string) => Promise.reject(new Error(String(calls.push(command)))) });
    });
    await openDevices(page);
    await page.getByRole('link', { name: 'nas', exact: true }).click();
    await expect(page.getByRole('heading', { level: 1, name: 'nas' })).toBeVisible();
    await page.goto(`${HUB_ORIGIN}/settings`, { waitUntil: 'domcontentloaded' });
    await expect(page.getByRole('heading', { level: 2, name: 'API keys' })).toBeVisible();

    expect(requested.filter((path) => updater.includes(path))).toEqual([]);
    expect(requested.some((path) => path.startsWith('/_app/immutable/'))).toBe(true);
    expect(await page.evaluate(() => Reflect.get(window, 'animaTauriCalls') as string[])).toEqual([]);
  });

  test('UPD-001: the same check sees the updater chunk and the `invoke` calls once the page runs in the desktop shell', async ({ page, context }) => {
    // The control: in the shell (`isTauri`) the client starts the updater, so the check above cannot pass for want of looking.
    const updater = updaterChunkPaths();
    const requested: string[] = [];
    context.on('request', (request) => requested.push(new URL(request.url()).pathname));
    await context.addInitScript(() => {
      const calls: string[] = [];
      Reflect.set(window, 'animaTauriCalls', calls);
      Reflect.set(window, 'isTauri', true);
      Reflect.set(window, '__TAURI_INTERNALS__', { invoke: (command: string) => Promise.reject(new Error(String(calls.push(command)))), transformCallback: () => 0 });
    });
    await page.goto(`${HUB_ORIGIN}/sign-in`, { waitUntil: 'domcontentloaded' });
    await expect.poll(() => requested.filter((path) => updater.includes(path)).length).toBeGreaterThan(0);
    expect((await page.evaluate(() => Reflect.get(window, 'animaTauriCalls') as string[])).length).toBeGreaterThan(0);
  });
});
