<script lang="ts">
  import { untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { checksApi, type DeviceCheck } from '$lib/checks/api';
  import { devicesApi } from '$lib/devices/api';
  import { DEFAULT_LOG_FILTER } from '$lib/events/log';
  import { LogFeed } from '$lib/events/panel/feed.svelte';
  import LogList from '$lib/events/panel/LogList.svelte';
  import type { CheckFormValues } from '$lib/forms/check/model/values';
  import type { DeviceFormValues } from '$lib/forms/device/values';
  import { useHub } from '$lib/hub/context';
  import type { HubResult } from '$lib/hub/hub';
  import { press } from '$lib/motion/roles/press';
  import { sessionsApi } from '$lib/sessions/api';
  import { useSessions } from '$lib/sessions/sessions.svelte';
  import ChecksPanel from './checks/ChecksPanel.svelte';
  import { Detail } from './detail.svelte';
  import Header from './header/Header.svelte';
  import OverviewPanel from './overview/OverviewPanel.svelte';
  import { Pages } from './pages.svelte';
  import { saveCheck, saveDevice } from './save';
  import type { DeviceSession } from './sessions/sessions';
  import SessionsPanel from './sessions/SessionsPanel.svelte';
  import SettingsPanel from './settings/SettingsPanel.svelte';
  import TabPanel from './tabs/TabPanel.svelte';
  import Tabs from './tabs/Tabs.svelte';
  import { placeSearch, readPlace, type SettingsSection, type TabName, type TabPlace } from './tabs/tabs';
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  // Only the route reaches the hub; the panels take data and callbacks (D-1).
  const hub = useHub();
  const sessions = useSessions();
  const devices = devicesApi(hub);
  const checks = checksApi(hub);
  const connections = sessionsApi(hub);
  const now = () => new Date();
  const SESSION_PAGE = 50;
  // The layout mounts a new page for each path, so the ID never changes under this one.
  const id = untrack(() => data.id);
  const detail = new Detail(hub, id);
  const sessionPages = new Pages<DeviceSession>((after) => connections.history(id ?? '', SESSION_PAGE, after));
  const feed = new LogFeed(hub, now);
  feed.filter = { ...DEFAULT_LOG_FILTER, since: 'all', deviceId: id };

  let revision = $state(0);
  const place = $derived(readPlace(page.url.searchParams));
  const device = $derived(detail.device);
  const checksLoading = $derived(detail.checks === null && detail.checksError === null);

  $effect(() => {
    detail.start();
    return () => {
      detail.stop();
      sessionPages.stop();
      feed.stop();
    };
  });

  /** D-9: a panel's data loads the first time its tab shows. */
  const started: TabName[] = [];
  $effect(() => {
    const tab = place.tab;
    if (id === null || started.includes(tab)) return;
    started.push(tab);
    if (tab === 'overview' || tab === 'checks') void detail.loadChecks();
    else if (tab === 'sessions') void sessionPages.load();
    else if (tab === 'logs') feed.start();
  });

  /** D-9: the tab and section go to the URL, so Back and a reload restore them. */
  function go(next: TabPlace): void {
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); only the query is added
    void goto(`${resolve('/devices')}/${encodeURIComponent(page.params.id ?? '')}${placeSearch(next)}`, { keepFocus: true, noScroll: true });
  }

  function settings(section: SettingsSection | null): void {
    go({ tab: 'settings', section });
  }

  function connect(button: HTMLElement): void {
    if (device === null) return;
    sessions.openSession({ deviceId: device.id, deviceName: device.name, protocol: 'SSH', origin: button.getBoundingClientRect() });
  }

  async function saveSettings(values: DeviceFormValues): Promise<HubResult<unknown>> {
    if (device === null) return { ok: true, value: undefined };
    const saved = await saveDevice(device, values, devices);
    detail.setDevice(saved.device);
    if (saved.result.ok) revision += 1;
    return saved.result;
  }

  async function acceptHostKey(fingerprint: string): Promise<HubResult<unknown>> {
    const result = await connections.acceptHostKey(id ?? '', fingerprint);
    if (result.ok && detail.device !== null) detail.setDevice({ ...detail.device, ssh: { ...detail.device.ssh, hostKey: result.value } });
    return result;
  }

  async function clearCredential(): Promise<HubResult<unknown>> {
    const result = await devices.clearSshCredential(id ?? '');
    if (result.ok && detail.device !== null) {
      detail.setDevice({ ...detail.device, ssh: { ...detail.device.ssh, credentialStored: result.value.credentialStored } });
    }
    return result;
  }

  /** DEV-014: confirmed in the Settings tab; the hub deletes the rest (DEV-015). */
  async function deleteDevice(): Promise<HubResult<unknown>> {
    const result = await devices.delete(id ?? '');
    if (result.ok) void goto(resolve('/devices'));
    return result;
  }

  async function putCheck(pending: Promise<HubResult<DeviceCheck>>): Promise<HubResult<unknown>> {
    const result = await pending;
    if (result.ok) detail.putCheck(result.value);
    return result;
  }

  async function dropCheck(check: DeviceCheck): Promise<HubResult<unknown>> {
    const result = await checks.delete(check.id);
    if (result.ok) detail.dropCheck(check.id);
    return result;
  }
</script>

<section class="detail">
  <Header {device} onConnect={connect} onSettings={settings} />
  {#if page.state.credentialError !== undefined}
    <p class="notice" role="alert">
      The device was added, but its credential was not stored: {page.state.credentialError} Enter it again in Settings.
    </p>
  {/if}
  {#if detail.missing}
    <h1>Device not found</h1>
    <p>There is no device at this address. It may have been deleted.</p>
  {:else if detail.error !== null}
    <div class="notice" role="alert">
      <p>{detail.error.message}</p>
      <button type="button" onclick={() => void detail.load()} {@attach press()}>Retry</button>
    </div>
  {:else if device !== null}
    {#if detail.liveError !== null}<p class="notice" role="status">Live updates stopped: {detail.liveError.message}</p>{/if}
    <Tabs selected={place.tab} onSelect={(tab) => { go({ tab, section: null }); }} />
    {#if place.tab === 'overview'}
      <TabPanel name="overview" loading={checksLoading}>
        <OverviewPanel {device} checks={detail.checks ?? []} />
      </TabPanel>
    {:else if place.tab === 'checks'}
      <TabPanel name="checks" loading={checksLoading}>
        {#if detail.checksError !== null}
          <div class="notice" role="alert">
            <p>{detail.checksError.message}</p>
            <button type="button" onclick={() => void detail.loadChecks()} {@attach press()}>Retry</button>
          </div>
        {:else}
          <ChecksPanel
            checks={detail.checks ?? []}
            history={detail.history}
            {now}
            onHistory={(id) => { detail.openHistory(id); }}
            onCloseHistory={() => { detail.closeHistory(); }}
            onPause={(check, paused) => putCheck(checks.setPaused(check.id, paused))}
            onSave={(check, values: CheckFormValues) => putCheck(saveCheck(device.id, check, values, checks))}
            onDelete={dropCheck}
          />
        {/if}
      </TabPanel>
    {:else if place.tab === 'sessions'}
      <TabPanel name="sessions" loading={sessionPages.items === null && sessionPages.error === null}>
        {#if sessionPages.error !== null}
          <div class="notice" role="alert">
            <p>{sessionPages.error.message}</p>
            <button type="button" onclick={() => void sessionPages.load()} {@attach press()}>Retry</button>
          </div>
        {:else}
          <SessionsPanel pages={sessionPages} {now} />
        {/if}
      </TabPanel>
    {:else if place.tab === 'logs'}
      <!-- The log list shows its own errors and Retry. -->
      <TabPanel name="logs" loading={feed.log === null && feed.error === null}>
        <LogList {feed} {now} />
      </TabPanel>
    {:else}
      <TabPanel name="settings" loading={false}>
        <SettingsPanel
          {device}
          {revision}
          section={place.section}
          onSave={saveSettings}
          onTest={connections.testConnection}
          onAcceptHostKey={acceptHostKey}
          onClearCredential={clearCredential}
          onCancel={() => { revision += 1; }}
          onDelete={deleteDevice}
        />
      </TabPanel>
    {/if}
  {/if}
</section>

<style>
  .detail {
    min-inline-size: 0;
  }

  h1 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-xl);
  }

  .notice {
    margin: 0 0 var(--space-4);
    color: var(--text);
  }

  .notice p {
    margin: 0 0 var(--space-2);
  }

  button {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
