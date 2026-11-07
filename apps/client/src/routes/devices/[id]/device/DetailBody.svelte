<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { checksApi } from '$lib/checks/api';
  import { desktopWebView } from '$lib/desktop/webView/webView';
  import type { DeviceDetail } from '$lib/devices/api';
  import { devicesApi } from '$lib/devices/client';
  import LiveNotice from '$lib/hub/notice/LiveNotice.svelte';
  import LogList from '$lib/events/panel/LogList.svelte';
  import type { CheckFormValues } from '$lib/forms/check/model/values';
  import { useHub } from '$lib/hub/context';
  import { press } from '$lib/motion/roles/press';
  import { sessionsApi } from '$lib/sessions/api';
  import { useSessions } from '$lib/sessions/sessions.svelte';
  import { openWebInterface } from '$lib/sessions/web/open/open';
  import { webInterfacesApi } from '$lib/web-interfaces/api';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import ChecksPanel from '../checks/ChecksPanel.svelte';
  import type { DeviceId } from '../device-id';
  import { Detail } from '../detail.svelte';
  import Header from '../header/Header.svelte';
  import OverviewPanel from '../overview/OverviewPanel.svelte';
  import OverviewSkeleton from '../overview/OverviewSkeleton.svelte';
  import { saveCheck } from '../save';
  import SessionsPanel from '../sessions/SessionsPanel.svelte';
  import { settingsCalls } from '../settings/calls';
  import SettingsPanel from '../settings/SettingsPanel.svelte';
  import TabPanel from '../tabs/TabPanel.svelte';
  import Tabs from '../tabs/Tabs.svelte';
  import { placeSearch, readPlace, type SettingsSection, type TabPlace } from '../tabs/tabs';
  import DeviceMissing from './DeviceMissing.svelte';

  /** The device's ID, parsed at the route edge; the route mounts this only for a well-formed one (D-14). */
  const { id }: { id: DeviceId } = $props();

  // Only the route reaches the hub; the panels take data and callbacks (D-1).
  const hub = useHub();
  const sessions = useSessions();
  const devices = devicesApi(hub);
  const checks = checksApi(hub);
  const connections = sessionsApi(hub);
  const now = () => new Date();
  // The layout mounts a new page for each path, so the ID never changes under this one.
  // svelte-ignore state_referenced_locally
  const detail = new Detail(hub, id, now);
  const { metrics, sessionPages, feed } = detail;

  let revision = $state(0);
  const settingsHub = { devices, sessions: connections, webInterfaces: webInterfacesApi(hub) };
  // svelte-ignore state_referenced_locally
  const settingsCall = settingsCalls(id, settingsHub, detail, () => {
    revision += 1;
  });
  const place = $derived(readPlace(page.url.searchParams));
  const device = $derived(detail.device);
  const headerLoading = $derived(device === null && detail.error === null && !detail.missing);
  const checksLoading = $derived(detail.checks === null && detail.checksError === null);
  const metricsSource = $derived(device?.metricsSource ?? null);

  $effect(() => {
    detail.start();
    return () => {
      detail.stop();
    };
  });

  $effect(() => {
    detail.showTab(place.tab);
  });

  $effect(() => {
    detail.showMetrics(place.tab, place.range);
  });

  /** D-9: the tab and section go to the URL, so Back and a reload restore them. */
  function go(next: TabPlace): void {
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); only the query is added
    void goto(`${resolve('/devices')}/${encodeURIComponent(page.params.id ?? '')}${placeSearch(next)}`, { keepFocus: true, noScroll: true });
  }

  function settings(section: SettingsSection | null): void {
    go({ tab: 'settings', section, range: 'HOUR' });
  }

  /** Opens a tab for the protocol of the header button that grows from it (RA-030, MOT-005). */
  function connect(protocol: SessionProtocol, button: HTMLElement): void {
    if (device === null) return;
    sessions.openSession({ deviceId: device.id, deviceName: device.name, protocol, origin: button.getBoundingClientRect() });
  }

  /** Opens the interface as its setting says: a web tab that grows from the button, or the system browser (WEB-003, WEB-011). */
  function openWeb(webInterface: DeviceDetail['webInterfaces'][number], button: HTMLElement): void {
    if (device === null) return;
    void openWebInterface(sessions, desktopWebView, { device, webInterface, origin: button.getBoundingClientRect() });
  }
</script>


{#snippet checksFailure(message: string)}
  <div class="notice" role="alert">
    <p>{message}</p>
    <button type="button" onclick={() => void detail.loadChecks()} {@attach press()}>Retry</button>
  </div>
{/snippet}

{#snippet overviewSkeleton()}
  <OverviewSkeleton range={place.range} source={metricsSource ?? 'NONE'} ssh={device?.ssh ?? null} {now} />
{/snippet}

<section class="detail">
  <Header {device} loading={headerLoading} onConnect={connect} onOpenWeb={openWeb} onSettings={settings} />
  {#if page.state.credentialError !== undefined}
    <p class="notice" role="alert">
      The device was added, but its credential was not stored: {page.state.credentialError} Enter it again in Settings.
    </p>
  {/if}
  {#if detail.missing}
    <DeviceMissing />
  {:else if detail.error !== null}
    <div class="notice" role="alert">
      <p>{detail.error.message}</p>
      <button type="button" onclick={() => void detail.load()} {@attach press()}>Retry</button>
    </div>
  {:else if device !== null}
    <LiveNotice message={detail.liveError?.message ?? null} gap="end" />
    <Tabs selected={place.tab} onSelect={(tab) => { go({ tab, section: null, range: 'HOUR' }); }} />
    {#if place.tab === 'overview'}
      <TabPanel name="overview" loading={checksLoading} skeleton={overviewSkeleton}>
        {#if detail.checksError !== null}
          {@render checksFailure(detail.checksError.message)}
        {:else}
          <OverviewPanel
            {device}
            checks={detail.checks ?? []}
            {metrics}
            range={place.range}
            {now}
            onRange={(range) => { go({ tab: 'overview', section: null, range }); }}
            onRetry={() => void metrics.load(place.range, device.metricsSource)}
            onSettings={() => { settings('general'); }}
          />
        {/if}
      </TabPanel>
    {:else if place.tab === 'checks'}
      <TabPanel name="checks" loading={checksLoading}>
        {#if detail.checksError !== null}
          {@render checksFailure(detail.checksError.message)}
        {:else}
          <ChecksPanel
            checks={detail.checks ?? []}
            history={detail.history}
            {now}
            onHistory={(id) => { detail.openHistory(id); }}
            onCloseHistory={() => { detail.closeHistory(); }}
            onPause={(check, paused) => detail.applyCheck(checks.setPaused(check.id, paused))}
            onSave={(check, values: CheckFormValues) => detail.applyCheck(saveCheck(device.id, check, values, checks))}
            onDelete={(check) => detail.deleteCheck(check)}
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
          calls={settingsCall}
          onTest={connections.testConnection}
          onCancel={() => { revision += 1; }}
          onDelete={() => detail.deleteDevice(() => void goto(resolve('/devices')))}
        />
      </TabPanel>
    {/if}
  {/if}
</section>

<style>
  .detail {
    min-inline-size: 0;
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
