<script lang="ts">
  import type { SessionsApi } from '../api';
  import { overlayCovers } from '$lib/overlay/overlay.svelte';
  import { useSessions } from '../sessions.svelte';
  import { openScreen } from '../screen/screen';
  import RemoteView from '../view/remote/RemoteView.svelte';
  import SessionView from '../view/SessionView.svelte';
  import WebTab from '$lib/web-interfaces/tab/WebTab.svelte';
  import { panelId, tabElementId } from './tablist';

  const { api }: { api: Pick<SessionsApi, 'endSession' | 'testConnection' | 'acceptHostKey' | 'acceptServerCertificate' | 'setCredential'> } = $props();

  const sessions = useSessions();
  /** The app window has the focus and the page is shown, so the selected pane may take the keyboard (D-8). */
  let windowFocused = $state(document.hasFocus());
  let pageShown = $state(document.visibilityState === 'visible');
  const focused = $derived(windowFocused && pageShown);
</script>

<svelte:window onfocus={() => (windowFocused = true)} onblur={() => (windowFocused = false)} />
<svelte:document onvisibilitychange={() => (pageShown = document.visibilityState === 'visible')} />

<!--
  One panel per open tab, all kept mounted so a hidden session stays alive (D-11); only the
  selected one shows, and a hidden one is inert with its keyboard unbound (D-8). No motion
  here: a terminal or a remote display is never animated (MAT-008).
-->
<div class="panes" hidden={sessions.active === null}>
  {#each sessions.tabs as tab (tab.id)}
    {@const visible = sessions.active?.id === tab.id}
    <div
      class="pane"
      role="tabpanel"
      id={panelId(tab.id)}
      aria-labelledby={tabElementId(tab.id)}
      hidden={!visible}
      inert={!visible}
    >
      {#if tab.protocol === 'WEB'}
        <WebTab
          webInterface={tab.web}
          device={{ id: tab.deviceId, name: tab.deviceName, status: tab.status }}
          tabId={tab.id}
          {visible}
          overlayOpen={overlayCovers()}
        />
      {:else if tab.protocol === 'SSH'}
        <SessionView
          device={{ id: tab.deviceId, name: tab.deviceName }}
          start={sessions.startIn(tab.id)}
          {api}
          {openScreen}
          {visible}
          {focused}
          onretitle={(title) => sessions.retitle(tab.id, title)}
        />
      {:else}
        <RemoteView
          device={{ id: tab.deviceId, name: tab.deviceName }}
          protocol={tab.protocol}
          start={sessions.remoteStartIn(tab.id)}
          {api}
          {visible}
          {focused}
        />
      {/if}
    </div>
  {/each}
</div>

<style>
  /* Shares the page area's grid cell in the root layout, over the pages. */
  .panes {
    grid-area: 1 / 1;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-inline-size: 0;
    background: var(--surface-base);
  }

  .panes[hidden],
  .pane[hidden] {
    display: none;
  }

  .pane {
    grid-area: 1 / 1;
    min-block-size: 0;
    min-inline-size: 0;
  }
</style>
