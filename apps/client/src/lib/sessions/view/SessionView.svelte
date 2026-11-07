<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import type { SessionsApi } from '../api';
  import type { SessionEnding } from '../ending';
  import { checkLink, linkRefusalText, openLink } from '../screen/remote/links';
  import type { ScreenHooks, ScreenOpener, TerminalScreen } from '../screen/screen';
  import type { CellSize } from '../tunnel/terminal/codec';
  import { NO_CLIPBOARD, clipboardRules } from './clipboard/clipboard';
  import { displayInput, type Size } from './display';
  import FilesPanel from '../files/FilesPanel.svelte';
  import EndedPanel from './EndedPanel.svelte';
  import HostKeyModal from './hostkey/HostKeyModal.svelte';
  import KeysMenu from './menu/KeysMenu.svelte';
  import { KEY_SEQUENCES, type KeySequence } from './menu/sequences';
  import FindBar from './pane/FindBar.svelte';
  import LinkConfirm from './pane/LinkConfirm.svelte';
  import Notice from './pane/Notice.svelte';
  import { SessionConnection, progressOf, type Stage } from './session.svelte';
  import type { SessionStart } from './start';
  import ConnectionSteps from './steps/ConnectionSteps.svelte';

  interface Props {
    device: { readonly id: string; readonly name: string };
    /** Starts a session on `device` with a new ticket; see `sessionStart` in `./start`. */
    start: SessionStart;
    api: Pick<SessionsApi, 'endSession' | 'testConnection' | 'acceptHostKey'>;
    /** The screen factory: opens the xterm.js terminal in the pane. Production passes `openScreen` from `../screen/screen`. */
    openScreen: ScreenOpener;
    /** The pane is on screen (its tab is selected). */
    visible: boolean;
    /** The pane should take the keyboard (its tab is selected and the app has focus). */
    focused: boolean;
    /** Hears the final ending of each attempt (after `endSession` answered, D-4). */
    onended?: (ending: SessionEnding) => void;
    /** Hears the remote shell's window title, cleaned (K4 §8.7); `''` when a session starts or ends. */
    onretitle?: (title: string) => void;
  }

  const { device, start, api, openScreen, visible, focused, onended, onretitle }: Props = $props();
  const COPY_FAILED = 'Could not copy the text to your clipboard.';
  const PASTE_FAILED = 'Could not read your clipboard. Allow clipboard access, or paste with the keyboard.';
  const OVERFLOW = 'Press Ctrl+L to redraw: output arrived faster than it could be drawn, so some was skipped.';
  /** The size of the first `resize` when the pane could not be fitted yet (a hidden pane); a fit corrects it. */
  const UNFITTED: CellSize = { cols: 80, rows: 24 };

  // The screen and the pane size are read in event handlers only, so they are plain variables, not state.
  let screen: TerminalScreen | undefined;
  let paneSize: Size | undefined;
  const connection = new SessionConnection(
    (display, events) => start(display, { ...events, size: () => screen?.size() ?? UNFITTED, onbytes: (bytes) => screen?.write(bytes) }),
    { endSession: (id) => api.endSession(id) },
    (ending) => {
      onretitle?.('');
      onended?.(ending);
    },
  );
  let notice: string | undefined = $state();
  let dismissed: Stage | undefined = $state.raw();
  let finding = $state(false);
  let filing = $state(false);
  /** An address the device asked to open, waiting for the operator's confirmation (K4 §8.7). */
  let confirming: string | undefined = $state();

  const stage = $derived(connection.stage);
  const ready = $derived(connection.ready);
  /** What this attempt may do with the clipboard, from the policy read at its start (RA-033). */
  const rules = $derived(stage.kind === 'open' ? clipboardRules(stage.policy) : NO_CLIPBOARD);
  /** The parent gives this pane the keyboard; the terminal then takes the focus (D-8). */
  const wanted = $derived(visible && focused && ready);
  // A hidden tab's warning waits until its tab is shown, so it never covers or traps another tab (SEC-007).
  const hostKeyOpen = $derived(visible && stage.kind === 'ended' && stage.hostKeyChanged && dismissed !== stage);

  const hooks: ScreenHooks = {
    oninput: (bytes) => connection.tunnel?.send(bytes),
    onresize: (size) => connection.tunnel?.resize(size),
    ontitle: (title) => onretitle?.(title),
    oncopy: (text) => copyToSystem(text),
    allows: () => rules,
    onfind: () => (finding = true),
    onoverflow: () => (notice = OVERFLOW),
    onlink: (address) => {
      const check = checkLink(address);
      confirming = check.ok ? check.address : undefined;
      if (!check.ok) notice = linkRefusalText(check.reason);
    },
  };

  function connect(css: Size) {
    const display = displayInput(css, window.devicePixelRatio, Intl.DateTimeFormat().resolvedOptions().timeZone);
    void connection.connect(display);
  }

  function reconnect() {
    dismissed = stage;
    notice = undefined;
    screen?.reset();
    connect(paneSize ?? { width: 0, height: 0 });
  }

  function copyToSystem(text: string) {
    navigator.clipboard.writeText(text).catch(() => {
      notice = COPY_FAILED;
    });
  }

  function sendKeys(sequence: KeySequence) {
    connection.tunnel?.send(sequence.bytes);
    screen?.focus();
  }

  function pasteFromClipboard() {
    navigator.clipboard.readText().then(
      (text) => {
        screen?.paste(text);
        screen?.focus();
      },
      () => {
        notice = PASTE_FAILED;
      },
    );
  }

  function closeFind() {
    finding = false;
    screen?.clearFind();
    screen?.focus();
  }

  function openConfirmed(address: string) {
    confirming = undefined;
    openLink(address);
  }

  /**
   * The terminal in the pane, and its size: the first measure opens the terminal, then starts the session at that
   * fit (D-7); later measures refit it, and the terminal reports a changed size in cells.
   */
  function attachPane(element: HTMLElement) {
    let live = true;
    const begin = async (css: Size) => {
      const opened = await openScreen(element, hooks);
      if (!live) {
        opened.dispose();
        return;
      }
      screen = opened;
      connect(css);
    };
    const observer = new ResizeObserver((entries) => {
      const box = entries.at(-1)?.contentRect;
      if (box === undefined) return;
      const first = paneSize === undefined;
      paneSize = { width: box.width, height: box.height };
      if (first) void begin(paneSize);
      else screen?.fit();
    });
    observer.observe(element);
    return () => {
      live = false;
      observer.disconnect();
      screen?.dispose();
      screen = undefined;
    };
  }

  // The terminal owns its keyboard (a hidden text field); it holds the focus only while the pane is shown, the app
  // has the focus and the socket is open (D-8).
  // The file panel belongs to one session: when the view is not ready its toggle is cleared too (C-3).
  $effect(() => {
    if (!ready) filing = false;
    if (wanted) screen?.focus();
    else screen?.blur();
  });

  $effect(() => () => {
    connection.close();
  });
</script>

<section class="session" hidden={!visible} aria-label="SSH session on {device.name}">
  <div class="toolbar">
    <KeysMenu disabled={!ready} entries={KEY_SEQUENCES} onkeys={sendKeys} onpaste={pasteFromClipboard} pasteOff={rules.pasteOff} />
    <button
      type="button"
      class="find"
      aria-expanded={finding}
      disabled={!ready}
      title="Find in the terminal (Cmd+F, Ctrl+Shift+F)"
      onclick={() => (finding ? closeFind() : (finding = true))}
      {@attach press()}
    >
      <Icon name="nf-md-magnify" />
      <span>Find</span>
    </button>
    <button type="button" class="find" aria-expanded={filing} disabled={!ready} title="Files on the device" onclick={() => (filing = !filing)} {@attach press()}><Icon name="nf-md-folder" /><span>Files</span></button>
    <span class="device">{device.name}</span>
    {#if ready && rules.notice !== undefined}<span class="policy" data-policy title={rules.notice}>{rules.notice}</span>{/if}
    <!-- Find and notices lie over the toolbar row, which has a fixed height, so the terminal never moves or resizes for them (MOT-007, UI-009). -->
    {#if finding && ready}
      <div class="cover">
        <FindBar onfind={(text, backwards, incremental) => screen?.find(text, backwards, incremental) ?? false} onclose={closeFind} />
      </div>
    {:else if notice !== undefined}
      <div class="cover">
        <Notice text={notice} ondismiss={() => (notice = undefined)} />
      </div>
    {/if}
  </div>
  <div class="stage">
    <!-- Before the pane in the DOM so Tab reaches it (the terminal keeps Tab); `order` keeps it on the right. -->
    {#if filing && stage.kind === 'open' && ready}<FilesPanel sessionId={stage.sessionId} />{/if}
    <!-- The terminal draws here; its own hidden text field takes the keyboard (D-8). -->
    <div class="pane" role="group" aria-label="Terminal of {device.name}" {@attach attachPane}></div>
    {#if !ready}
      <!-- Top-aligned, steps first: the ended text appears below the steps, so nothing above it moves (MV-004). -->
      <div class="panel">
        {#if !progressOf(stage).connected}
          <ConnectionSteps progress={progressOf(stage)} />
        {/if}
        {#if stage.kind === 'ending' || stage.kind === 'ended'}
          <EndedPanel ending={stage.ending} settled={stage.kind === 'ended'} onreconnect={reconnect} />
        {/if}
      </div>
    {/if}
  </div>
</section>

<LinkConfirm address={confirming} onopen={openConfirmed} oncancel={() => (confirming = undefined)} />

{#if hostKeyOpen}
  <HostKeyModal {device} {api} onclose={() => (dismissed = stage)} onreconnect={reconnect} />
{/if}

<style>
  .session {
    display: flex;
    font-family: var(--font-ui);
    flex-direction: column;
    block-size: 100%;
    min-block-size: 0;
    min-inline-size: 0;
    background: var(--bg);
    color: var(--text);
  }

  .session[hidden] {
    display: none;
  }

  /*
   * Solid and above the display, never over it (MAT-008). A token line height, not `normal`, so the bar is as tall
   * before the bundled fonts load as after; with `normal` it grew 6 px when they did (MOT-007, R2).
   */
  .toolbar {
    position: relative;
    box-sizing: border-box;
    display: flex;
    block-size: calc(var(--target-min-desktop) + 2 * var(--space-2) + var(--border-thin));
    line-height: var(--leading-normal);
    flex: none;
    align-items: center;
    gap: var(--space-3);
    min-inline-size: 0;
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-thin) solid var(--border);
    background: var(--surface);
  }

  /* Lies over the whole toolbar row, bounded by `inset: 0` (the toolbar itself is not clipped, so the Keys menu can reach out of it): the find bar, or a one-line message with a way to dismiss it. */
  .cover {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-inline-size: 0;
    padding: var(--space-2) var(--space-3);
    background: var(--surface);
  }

  /* The bar never sets the pane's width: the name and the policy give way first, and the controls keep their size (UI-009). */
  .device,
  .policy {
    min-inline-size: 0;
    overflow: hidden;
    font-size: var(--text-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .device {
    font-family: var(--font-mono);
  }

  .policy {
    flex: 1;
    color: var(--text-muted);
  }

  .find {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
  }

  .find:disabled {
    color: var(--text-muted);
  }

  .find:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .stage {
    position: relative;
    display: flex;
    flex: 1;
    min-block-size: 0;
  }

  /* The pane's size comes from the layout only, never from the terminal inside it, so a fit cannot loop (D-7). */
  .pane {
    position: relative;
    flex: 1;
    min-inline-size: 0;
    box-sizing: border-box;
    overflow: hidden;
    /* Wider than the focus ring, so the ring never covers a glyph in column 0 or row 0 (UI-009, MAT-008). The fit add-on measures inside it. */
    padding: var(--space-1);
  }

  /*
   * The terminal is never animated, transformed, filtered or covered by a material (MAT-008). The ring is drawn
   * after it, over its edge, without taking pointer input (UI-009, UI-012).
   */
  .pane:focus-within::after {
    content: '';
    position: absolute;
    inset: 0;
    pointer-events: none;
    box-shadow: inset 0 0 0 var(--border-focus) var(--accent);
  }

  /* Shown only while the terminal is not: before ready and after the end (D-7). A narrow pane scrolls it, never the page (UI-009). */
  .panel {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: flex-start;
    gap: var(--space-4);
    overflow: auto;
    padding: var(--space-6);
    background: var(--bg);
  }
</style>
