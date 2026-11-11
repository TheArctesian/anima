<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import type { DeviceDetail } from '$lib/devices/api';
  import LiveNotice from '$lib/hub/notice/LiveNotice.svelte';
  import LogList from '$lib/events/panel/LogList.svelte';
  import type { CheckFormValues } from '$lib/checks/form/model/values';
  import { thisComputerShell } from '$lib/desktop/agent/shell';
  import { screenSharingCalls } from '$lib/devices/screen-sharing/calls';
  import { metricsApi } from '$lib/metrics/api';
  import { useHub } from '$lib/hub/context';
  import { press } from '$lib/motion/roles/press';
  import { useSessions } from '$lib/sessions/sessions.svelte';
  import { openWebInterface } from '$lib/web-interfaces/open/open';
  import { webViewShell } from '$lib/web-interfaces/shell/client-shell';
  import { vpnApi } from '$lib/vpn/api';
  import { VpnLive } from '$lib/vpn/live.svelte';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import ChecksPanel from '../checks/ChecksPanel.svelte';
  import type { DeviceId } from '../device-id';
  import { Detail } from '../detail.svelte';
  import Header from '../header/Header.svelte';
  import OverviewPanel from '../overview/OverviewPanel.svelte';
  import OverviewSkeleton from '../overview/OverviewSkeleton.svelte';
  import SessionsPanel from '../sessions/SessionsPanel.svelte';
  import SettingsPanel from '../settings/SettingsPanel.svelte';
  import TabPanel from '../tabs/TabPanel.svelte';
  import Tabs from '../tabs/Tabs.svelte';
  import { placeSearch, readPlace, type SettingsSection, type TabPlace } from '../tabs/tabs';
  import { detailCalls } from './calls';
  import DeviceMissing from './DeviceMissing.svelte';

  /** The device's ID, parsed at the route edge; the route mounts this only for a well-formed one (D-14). */
  const { id }: { id: DeviceId } = $props();

  // The panels take data and callbacks; `Detail` and `detailCalls` reach the hub (D-1).
  const hub = useHub();
  const sessions = useSessions();
  const screenSharing = { calls: screenSharingCalls(hub), onEnabled: (next: DeviceDetail) => { detail.setDevice(next); } };
  const now = () => new Date();
  const agentCalls = metricsApi(hub);
  const agentShell = thisComputerShell();
  const agent = $derived({ calls: agentCalls, origin: hub.address.origin, reveal: page.url.hash === '#agent', shell: agentShell });
  // The layout mounts a new page for each path, so the ID never changes under this one.
  // svelte-ignore state_referenced_locally
  const detail = new Detail(hub, id, now);
  const { metrics, sessionPages, feed } = detail;

  let revision = $state(0);
  const calls = detailCalls(
    hub,
    detail,
    () => {
      revision += 1;
    },
    () => void goto(resolve('/devices')),
  );
  const place = $derived(readPlace(page.url.searchParams));
  const device = $derived(detail.device);
  const headerLoading = $derived(device === null && detail.error === null && !detail.missing);
  const checksLoading = $derived(detail.checks === null && detail.checksError === null);
  const metricsSource = $derived(device?.metricsSource ?? null);

  // One VPN subscription for the page; the Overview card reads this device's node from it (VPN-005).
  const vpnLive = new VpnLive(vpnApi(hub));
  const vpn = $derived(vpnLive.picture?.nodes.find((node) => node.device?.id === device?.id) ?? null);

  $effect(() => {
    void vpnLive.load();
    return () => {
      vpnLive.stop();
    };
  });

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
    void goto(`${resolve('/devices')}/${encodeURIComponent(id)}${placeSearch(next)}`, { keepFocus: true, noScroll: true });
  }

  /** The Overview's way to the Agent panel on the Settings tab; the panel takes focus (MET-019). */
  function agentPanel(): void {
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); only the query and hash are added
    void goto(`${resolve('/devices')}/${encodeURIComponent(id)}${placeSearch({ tab: 'settings', section: null, range: 'HOUR' })}#agent`, { keepFocus: true, noScroll: true });
  }

  function settings(section: SettingsSection | null): void {
    go({ tab: 'settings', section, range: 'HOUR' });
  }

  /** Opens a tab for the protocol of the header button that grows from it (RA-030, MOT-005). */
  function connect(protocol: SessionProtocol, button: HTMLElement): void {
    if (device === null) return;
    sessions.openSession({ deviceId: device.id, deviceName: device.name, os: device.os, ...(device.distribution == null ? {} : { distribution: device.distribution }), protocol, origin: button.getBoundingClientRect() });
  }

  /** Opens the interface as its setting says: a web tab that grows from the button, or the system browser (WEB-003, WEB-011). */
  function openWeb(webInterface: DeviceDetail['webInterfaces'][number], button: HTMLElement): void {
    if (device === null) return;
    void openWebInterface(sessions, webViewShell(), { device, webInterface, origin: button.getBoundingClientRect() });
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
  <Header {device} loading={headerLoading} onConnect={connect} onOpenWeb={openWeb} {screenSharing} onSettings={settings} />
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
            {vpn}
            {now}
            onRange={(range) => { go({ tab: 'overview', section: null, range }); }}
            onRetry={() => void metrics.load(place.range, device.metricsSource)}
            onSettings={agentPanel}
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
            onPause={(check, paused) => detail.pauseCheck(check, paused)}
            onSave={(check, values: CheckFormValues) => detail.saveCheckValues(check, values)}
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
          calls={calls.settings}
          {agent}
          {now}
          onTest={calls.onTest}
          onCancel={() => { revision += 1; }}
          onDelete={calls.onDelete}
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
