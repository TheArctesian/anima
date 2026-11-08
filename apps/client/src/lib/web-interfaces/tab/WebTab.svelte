<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import { webViewShell } from '$lib/web-interfaces/shell/choice';
  import { codeOf, type Bounds, type WebViewShell } from '$lib/web-interfaces/shell/shell';
  import type { Status } from '$lib/hub/generated/graphql';
  import { useHub } from '$lib/hub/context';
  import { webTabApi, type WebTabApi } from './api';
  import BrowserPage from './BrowserPage.svelte';
  import DownPage from './DownPage.svelte';
  import {
    browserAllowed,
    changeOf,
    checksBeforeBrowser,
    failureText,
    LOAD_TIMEOUT_MS,
    startTab,
    tlsStatus,
    transition,
    viewShows,
    type WebStep,
    type WebTab,
    type WebTabEvent,
    type WebTabTarget,
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
    /** The clock, in milliseconds; tests pass their own (CQ-021). */
    now?: () => number;
  }

  const { webInterface, device, tabId, visible, overlayOpen, shell = webViewShell(), api = webTabApi(useHub()), now = Date.now }: Props = $props();

  const TEXT = {
    scheme: 'Anima opens only http and https web interfaces.',
    pin: 'The pinned fingerprint of this web interface cannot be read, so Anima does not open it.',
    browser: 'The system browser could not be opened.',
  } as const;

  /** The interface as it is now, for a new run: a new URL or pin starts one (H-4, WEB-008). */
  const snapshot = (): WebTabTarget =>
    untrack(() => ({ id: webInterface.id, deviceId: webInterface.deviceId, url: webInterface.url, pinned: webInterface.pinnedFingerprintSha256 }));

  let fixed: WebTabTarget = $state.raw(snapshot());
  /** Counts runs; an answer for an earlier run is dropped. */
  let runs = 0;
  let pinning = false;
  /** After a restart, events wait for the new view: a late one from the closed view must not reach this run. */
  let restarted = false;
  let tab: WebTab | undefined = $state.raw();
  let viewState = $state<ViewState>('none');
  let bounds: Bounds | undefined = $state.raw();
  let opening = $state(false);
  let notice: string | undefined = $state();
  let area: HTMLElement | undefined = $state();
  let reloadButton: HTMLButtonElement | undefined = $state();
  let reading: WebStep | undefined;

  /** Move to the tab `event` leads to; undefined (and no change) when it does not apply now. */
  function apply(event: WebTabEvent): WebTab | undefined {
    if (tab === undefined) return undefined;
    const next = transition(tab, event, now());
    if (next !== undefined) tab = next;
    return next;
  }

  // A tab keeps its shell and ID for its whole life.
  const view = nativeView({
    shell: untrack(() => shell),
    tabId: untrack(() => tabId),
    onfail: (reason) => apply({ event: 'failed', reason }),
    onstate: (state) => {
      viewState = state;
      if (state === 'open') restarted = false;
    },
  });

  /** Start a run of the tab for the interface as it is now. */
  function start() {
    runs += 1;
    fixed = snapshot();
    const down = untrack(() => device.status) === 'DOWN';
    tab = startTab({ url: fixed.url, pinned: fixed.pinned, pinning, down, now: now() });
  }

  const current = $derived(tab?.current);
  const shownUrl = $derived(current?.step === 'loaded' ? current.url : fixed.url);
  const tls = $derived(tlsStatus(fixed.url, shownUrl, tab?.pin ?? null, current ?? { step: 'reading' }));
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
        if (event.tabId === tabId && !restarted) apply({ event: 'shell', shell: event });
      })
      .then((stop) => {
        if (stopped) stop();
        else unlisten = stop;
      })
      .catch(() => apply({ event: 'failed', reason: 'listen' }));
    void shell
      .pinning()
      .catch(() => false)
      .then((answer) => {
        if (stopped) return;
        pinning = answer;
        start();
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

  // H-4: a new URL drops the pin and a new pin replaces it, so the view of the old one closes and the tab starts
  // again; the pin this tab accepted itself coming back from the hub changes nothing.
  $effect(() => {
    const { url, pinnedFingerprintSha256: pinned } = webInterface;
    untrack(() => {
      const change = tab === undefined ? 'same' : changeOf(fixed, url, pinned, tab.pin);
      if (change === 'adopt') fixed = { ...fixed, pinned };
      if (change !== 'restart') return;
      restarted = true;
      view.restart();
      start();
    });
  });

  // The view opens once, when the tab first has an area and a page to load.
  $effect(() => {
    if (tab?.mode !== 'EMBEDDED' || current === undefined || !viewShows(current) || viewState !== 'none') return;
    if (bounds === undefined || !hasArea(bounds)) return;
    view.open({ deviceId: fixed.deviceId, url: fixed.url, pin: tab.pin }, bounds);
  });

  // MAT-008: the native view sits above the page, so it is hidden whenever anything else must be seen.
  $effect(() => {
    const shown = visible && !overlayOpen && current !== undefined && viewShows(current);
    if (viewState === 'open') view.show(shown);
  });

  // WEB-010: a page still loading after the timeout shows the error page; the web view reports no such failure.
  $effect(() => {
    if (current?.step !== 'loading') return;
    const timer = setTimeout(() => apply({ event: 'tick' }), Math.max(0, current.since + LOAD_TIMEOUT_MS - now()));
    return () => {
      clearTimeout(timer);
    };
  });

  // System-browser mode reads the certificate first when there is no pin (D-9).
  $effect(() => {
    if (current?.step !== 'reading' || reading === current) return;
    const step = current;
    reading = step;
    // The answer applies only to the step that asked, never to a later run's (H-4).
    const ours = () => tab?.current === step;
    void shell
      .readCertificate(fixed.url)
      .then((presented) => ours() && apply({ event: 'read', presented }))
      .catch((reason: unknown) => ours() && apply({ event: 'failed', reason: codeOf(reason) }));
  });

  async function accept() {
    const sending = apply({ event: 'accept' });
    if (sending?.current.step !== 'accepting') return;
    const step = sending.current;
    // Exactly the fingerprint shown; the state sends it once (WEB-008).
    const result = await api.acceptWebCertificate(fixed.id, step.presented);
    // A restart while the hub answered is another run: its view is never pinned with this fingerprint (H-4).
    if (tab?.current !== step) return;
    const next = apply({ event: 'accepted', result });
    if (next?.pin !== step.presented || next.mode !== 'EMBEDDED') return;
    view.pin(step.presented);
    // The prompt is gone; focus goes to Reload, once it is enabled, rather than falling to the page.
    await tick();
    if (visible) reloadButton?.focus();
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
    const run = runs;
    opening = true;
    notice = undefined;
    try {
      if (pin !== null && checksBeforeBrowser(fixed.url, url, pin)) {
        const presented = await shell.readCertificate(url);
        // Whatever came while reading counts: another run, a stop or a prompt now, or a new pin (WEB-008).
        if (run !== runs || tab === undefined || !browserAllowed(tab.current) || tab.pin !== pin) return;
        // A different certificate stops the open, as in the web view.
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
    bind:reloadButton
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
              url={fixed.url}
              onaccept={accept}
              onconfirm={() => apply({ event: 'confirm' })}
              onback={() => apply({ event: 'back' })}
            />
          {:else if current.step === 'down'}
            <DownPage reason="{device.name} is Down, so its web interface cannot be reached. Reload when the device is back up." onreload={reload} />
          {:else if current.step === 'failed'}
            <DownPage reason={failureText(current.reason)} code={current.reason} onreload={reload} />
          {:else if current.step === 'refused'}
            <DownPage reason={TEXT[current.reason]} />
          {:else if current.step === 'reading' || current.step === 'browser'}
            <BrowserPage label={webInterface.label} url={fixed.url} pin={tab?.pin ?? null} {opening} onopen={openBrowser} />
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
