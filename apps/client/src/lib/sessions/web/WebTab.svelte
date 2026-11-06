<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { codeOf, desktopWebView, type Bounds, type WebViewShell } from '$lib/desktop/webView/webView';
  import type { Status } from '$lib/hub/generated/graphql';
  import { useHub } from '$lib/hub/context';
  import { webTabApi, type WebTabApi } from './api';
  import BrowserPage from './BrowserPage.svelte';
  import DownPage from './DownPage.svelte';
  import {
    browserAllowed,
    checksBeforeBrowser,
    startTab,
    tlsStatus,
    transition,
    viewShows,
    type WebTab,
    type WebTabEvent,
  } from './tab';
  import Toolbar from './Toolbar.svelte';
  import TrustPage from './TrustPage.svelte';
  import { boundsOf, hasArea, nativeView, type ViewState } from './view';

  interface Props {
    webInterface: {
      readonly id: string;
      readonly deviceId: string;
      readonly url: string;
      readonly label: string;
      readonly pinnedFingerprintSha256: string | null;
    };
    device: { readonly id: string; readonly name: string; readonly status: Status };
    tabId: string;
    /** Whether this tab is the one on screen. */
    visible: boolean;
    /** Whether a menu, popover or modal lies over the tab: the native view would cover it (MAT-008). */
    overlayOpen: boolean;
    shell?: WebViewShell;
    api?: WebTabApi;
  }

  const { webInterface, device, tabId, visible, overlayOpen, shell = desktopWebView, api = webTabApi(useHub()) }: Props = $props();

  const TEXT = {
    failed: 'The page could not be loaded. Check that the web interface is running, then reload.',
    scheme: 'Anima opens only http and https web interfaces.',
    pin: 'The pinned fingerprint of this web interface cannot be read, so Anima does not open it.',
    browser: 'The system browser could not be opened.',
  } as const;

  let tab: WebTab | undefined = $state.raw();
  let viewState = $state<ViewState>('none');
  let bounds: Bounds | undefined = $state.raw();
  let opening = $state(false);
  let notice: string | undefined = $state();
  let area: HTMLElement | undefined = $state();
  let reading = false;

  /** Move to the tab `event` leads to; undefined (and no change) when it does not apply now. */
  function apply(event: WebTabEvent): WebTab | undefined {
    if (tab === undefined) return undefined;
    const next = transition(tab, event);
    if (next !== undefined) tab = next;
    return next;
  }

  // A tab keeps its shell and ID for its whole life.
  const view = nativeView({
    shell: untrack(() => shell),
    tabId: untrack(() => tabId),
    onfail: (reason) => apply({ event: 'failed', reason }),
    onstate: (state) => (viewState = state),
  });

  const current = $derived(tab?.current);
  const shownUrl = $derived(current?.step === 'loaded' ? current.url : webInterface.url);
  const tls = $derived(tlsStatus(webInterface.url, shownUrl, tab?.pin ?? null));
  const live = $derived(viewState === 'open' && current !== undefined && viewShows(current));
  const page = $derived(current !== undefined && !viewShows(current));
  /** A new page, or the second step of an accept, takes focus afresh. */
  const pageKey = $derived(current?.step === 'mismatch' && current.confirming ? 'confirming' : current?.step);

  function measure() {
    if (area === undefined) return;
    const next = boundsOf(area.getBoundingClientRect());
    bounds = next;
    if (hasArea(next)) view.place(next);
  }

  onMount(() => {
    let stopped = false;
    let unlisten: (() => void) | undefined;
    void shell
      .listen((event) => {
        if (event.tabId === tabId) apply({ event: 'shell', shell: event });
      })
      .then((stop) => {
        if (stopped) stop();
        else unlisten = stop;
      })
      .catch(() => apply({ event: 'failed', reason: 'listen' }));
    void shell.pinning().then((pinning) => {
      if (stopped) return;
      const down = device.status === 'DOWN';
      tab = startTab({ url: webInterface.url, pinned: webInterface.pinnedFingerprintSha256, pinning, down });
    });
    const observer = new ResizeObserver(measure);
    if (area !== undefined) observer.observe(area);
    window.addEventListener('resize', measure);
    return () => {
      stopped = true;
      observer.disconnect();
      window.removeEventListener('resize', measure);
      unlisten?.();
      view.close();
    };
  });

  // WEB-010: a device that turns Down replaces the page with the Down page.
  let lastStatus: Status | undefined;
  $effect(() => {
    const { status } = device;
    untrack(() => {
      if (status === 'DOWN' && lastStatus !== 'DOWN') apply({ event: 'down' });
      lastStatus = status;
    });
  });

  // The view opens once, when the tab first has an area and a page to load.
  $effect(() => {
    if (tab?.mode !== 'EMBEDDED' || current === undefined || !viewShows(current) || viewState !== 'none') return;
    if (bounds === undefined || !hasArea(bounds)) return;
    view.open({ deviceId: webInterface.deviceId, url: webInterface.url, pin: tab.pin }, bounds);
  });

  // MAT-008: the native view sits above the page, so it is hidden whenever anything else must be seen.
  $effect(() => {
    const shown = visible && !overlayOpen && current !== undefined && viewShows(current);
    if (viewState === 'open') view.show(shown);
  });

  // System-browser mode reads the certificate first when there is no pin (D-9).
  $effect(() => {
    if (current?.step !== 'reading' || reading) return;
    reading = true;
    void shell
      .readCertificate(webInterface.url)
      .then((presented) => apply({ event: 'read', presented }))
      .catch((reason: unknown) => apply({ event: 'failed', reason: codeOf(reason) }))
      .finally(() => (reading = false));
  });

  async function accept() {
    const sending = apply({ event: 'accept' });
    if (sending?.current.step !== 'accepting') return;
    const { presented } = sending.current;
    // Exactly the fingerprint shown; the state sends it once (WEB-008).
    const result = await api.acceptWebCertificate(webInterface.id, presented);
    const next = apply({ event: 'accepted', result });
    if (next?.mode === 'EMBEDDED' && next.pin === presented) view.pin(presented);
  }

  function reload() {
    if (current?.step === 'failed' || current?.step === 'down') {
      if (apply({ event: 'reload' }) !== undefined && viewState === 'open') view.go('RELOAD');
    } else {
      view.go('RELOAD');
    }
  }

  async function openBrowser() {
    if (tab === undefined || !browserAllowed(tab.current) || opening) return;
    const { pin } = tab;
    const url = shownUrl;
    opening = true;
    notice = undefined;
    try {
      if (pin !== null && checksBeforeBrowser(webInterface.url, url, pin)) {
        const presented = await shell.readCertificate(url);
        // A different certificate stops the open, as in the web view (WEB-008).
        if (presented !== pin) {
          apply({ event: 'mismatch', expected: pin, presented });
          return;
        }
      }
      await shell.openSystemBrowser(url);
    } catch {
      notice = TEXT.browser;
    } finally {
      opening = false;
    }
  }
</script>

<div class="web-tab">
  <Toolbar
    url={shownUrl}
    {tls}
    navigable={live}
    reloadable={live || current?.step === 'failed' || current?.step === 'down'}
    browsable={current !== undefined && browserAllowed(current) && !opening}
    poppable={live}
    onback={() => view.go('BACK')}
    onforward={() => view.go('FORWARD')}
    onreload={reload}
    onbrowser={openBrowser}
    onpopout={() => view.popOut()}
  />
  {#if notice !== undefined}
    <p class="notice" role="status">{notice}</p>
  {/if}
  <!-- The native web view is placed over this area: no motion, material or blur here (MAT-008). -->
  <div class="area" bind:this={area} data-view-area>
    {#if current !== undefined && (page || viewState === 'popped')}
      {#key pageKey}
        <div class="page" {@attach (node) => {
          if (visible) node.querySelector<HTMLElement>('[tabindex="-1"]')?.focus();
        }}>
          {#if viewState === 'popped' && !page}
            <p>{webInterface.label} is open in its own window.</p>
          {:else if current.step === 'untrusted' || current.step === 'accepting' || current.step === 'mismatch'}
            <TrustPage
              {current}
              changed={tab?.pin !== null}
              deviceName={device.name}
              label={webInterface.label}
              url={webInterface.url}
              onaccept={accept}
              onconfirm={() => apply({ event: 'confirm' })}
              onback={() => apply({ event: 'back' })}
            />
          {:else if current.step === 'down'}
            <DownPage reason="{device.name} is Down, so its web interface cannot be reached. Reload when the device is back up." onreload={reload} />
          {:else if current.step === 'failed'}
            <DownPage reason={TEXT.failed} code={current.reason} onreload={reload} />
          {:else if current.step === 'refused'}
            <DownPage reason={TEXT[current.reason]} />
          {:else if current.step === 'reading' || current.step === 'browser'}
            <BrowserPage label={webInterface.label} url={webInterface.url} pin={tab?.pin ?? null} {opening} onopen={openBrowser} />
          {/if}
        </div>
      {/key}
    {/if}
  </div>
</div>

<style>
  .web-tab {
    display: flex;
    flex-direction: column;
    block-size: 100%;
    min-block-size: 0;
    background: var(--bg);
  }

  .notice {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    background: var(--status-down-tint);
    color: var(--text);
    font-size: var(--text-sm);
  }

  .area {
    position: relative;
    flex: 1 1 auto;
    min-block-size: 0;
  }

  .page {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding: var(--space-8) var(--space-4);
    overflow-y: auto;
    color: var(--text);
  }

  .page > p {
    margin: 0;
  }
</style>
