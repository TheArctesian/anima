<script lang="ts">
  import type { SessionsApi } from '../api';
  import type { SessionEnding } from '../ending';
  import { paneKeyboard, receiveClipboard, type PaneKeyboard } from '../tunnel/input';
  import { displayInput, isMeasured, remoteSize, sizeSender, type Size, type SizeSender } from './display';
  import EndedPanel from './EndedPanel.svelte';
  import HostKeyModal from './hostkey/HostKeyModal.svelte';
  import { keyRoute, pasteText, sendCombination, showDisplay } from './input';
  import type { KeyCombination } from './keys';
  import KeysMenu from './menu/KeysMenu.svelte';
  import { SessionConnection, progressOf, type Stage } from './session.svelte';
  import type { SessionStart } from './start';
  import ConnectionSteps from './steps/ConnectionSteps.svelte';

  interface Props {
    device: { readonly id: string; readonly name: string };
    /** Starts a session on `device` with a new ticket; see `sessionStart` in `./start`. */
    start: SessionStart;
    api: Pick<SessionsApi, 'endSession' | 'testConnection' | 'acceptHostKey'>;
    /** The pane is on screen (its tab is selected). */
    visible: boolean;
    /** The pane should take the keyboard (its tab is selected and the app has focus). */
    focused: boolean;
    /** Hears the final ending of each attempt (after `endSession` answered, D-4). */
    onended?: (ending: SessionEnding) => void;
  }

  const { device, start, api, visible, focused, onended }: Props = $props();
  const COPY_FAILED = 'Could not copy the text to your clipboard.';
  const PASTE_FAILED = 'Could not read your clipboard. Allow clipboard access, or paste with the keyboard.';

  const connection = new SessionConnection(
    (display, events) => start(display, events),
    { endSession: (id) => api.endSession(id) },
    (ending) => onended?.(ending),
  );
  let pane: HTMLElement | undefined = $state();
  let keyboard: PaneKeyboard | undefined = $state();
  let paneSize: Size | undefined = $state();
  let startedAt: Size = { width: 0, height: 0 };
  let sizes: SizeSender | undefined;
  let notice: string | undefined = $state();
  let dismissed: Stage | undefined = $state();
  /** The latest gateway clipboard text that arrived while the pane was not bound; only the latest is kept (D-9). */
  let pendingCopy: string | undefined;

  const stage = $derived(connection.stage);
  const client = $derived(connection.tunnel?.client);
  const ready = $derived(connection.ready);
  const bound = $derived(visible && focused && ready);
  const hostKeyOpen = $derived(stage.kind === 'ended' && stage.hostKeyChanged && dismissed !== stage);

  function connect(css: Size) {
    const display = displayInput(css, window.devicePixelRatio, Intl.DateTimeFormat().resolvedOptions().timeZone);
    startedAt = { width: display.widthPx, height: display.heightPx };
    void connection.connect(display);
  }

  function reconnect() {
    dismissed = stage;
    notice = undefined;
    connect(paneSize ?? { width: 0, height: 0 });
  }

  function copyToSystem(text: string) {
    navigator.clipboard.writeText(text).catch(() => {
      notice = COPY_FAILED;
    });
  }

  function paste(event: ClipboardEvent) {
    if (client === undefined || !ready) return;
    event.preventDefault();
    pasteText(client, event.clipboardData?.getData('text/plain') ?? '');
  }

  function sendKeys(combination: KeyCombination) {
    if (client !== undefined) sendCombination(client, combination);
  }

  function pasteFromClipboard() {
    const target = client;
    navigator.clipboard.readText().then(
      (text) => {
        if (target !== undefined && target === client) pasteText(target, text);
      },
      () => {
        notice = PASTE_FAILED;
      },
    );
  }

  function resized(css: Size) {
    if (isMeasured(css)) sizes?.resized(remoteSize(css, window.devicePixelRatio));
  }

  /** The pane's keyboard, and its size: the first measure starts the session, later ones resize it (D-7). */
  function attachPane(element: HTMLElement) {
    keyboard = paneKeyboard(element);
    const observer = new ResizeObserver((entries) => {
      const box = entries.at(-1)?.contentRect;
      if (box === undefined) return;
      const css = { width: box.width, height: box.height };
      const first = paneSize === undefined;
      paneSize = css;
      if (first) connect(css);
      else resized(css);
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }

  // Each open tunnel gets a size sender that starts from the size it was opened with, and hears the pane's size
  // now in case it changed while the hub answered.
  $effect(() => {
    if (client === undefined) return;
    const target = client;
    const sender = sizeSender(startedAt, (size) => target.sendSize(size.width, size.height), {
      set: (callback, ms) => setTimeout(callback, ms),
      clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
    });
    sizes = sender;
    if (paneSize !== undefined) resized(paneSize);
    receiveClipboard(target, (text) => {
      if (bound) copyToSystem(text);
      else pendingCopy = text;
    });
    return () => {
      sender.cancel();
      sizes = undefined;
      target.onclipboard = null;
      pendingCopy = undefined;
    };
  });

  // The keyboard is bound only while the pane is visible, focused and ready; never to the document (D-8).
  $effect(() => {
    if (!bound || client === undefined || keyboard === undefined) return;
    const bindTo = keyboard;
    bindTo.bind(keyRoute(client));
    pane?.focus({ preventScroll: true });
    if (pendingCopy !== undefined) copyToSystem(pendingCopy);
    pendingCopy = undefined;
    return () => {
      bindTo.unbind();
    };
  });

  $effect(() => () => {
    connection.close();
  });
</script>

<section class="session" hidden={!visible} aria-label="SSH session on {device.name}">
  <div class="toolbar">
    <KeysMenu disabled={!ready} onkeys={sendKeys} onpaste={pasteFromClipboard} />
    <span class="device">{device.name}</span>
    {#if notice !== undefined}<span class="notice" role="status">{notice}</span>{/if}
  </div>
  <div class="stage">
    <!-- The keyboard listens here only; menus, panels and the modal are outside it (D-8). -->
    <!-- Focusable because the terminal takes the keyboard as a text field does (D-8). -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="pane"
      role="application"
      aria-label="Terminal of {device.name}"
      tabindex="0"
      bind:this={pane}
      onpaste={paste}
      {@attach attachPane}
    >
      {#if ready && client !== undefined}
        <div class="display" data-display {@attach showDisplay(client, window.devicePixelRatio)}></div>
      {/if}
    </div>
    {#if !ready}
      <div class="panel">
        {#if stage.kind === 'ending' || stage.kind === 'ended'}
          <EndedPanel ending={stage.ending} settled={stage.kind === 'ended'} onreconnect={reconnect} />
        {/if}
        {#if !progressOf(stage).connected}
          <ConnectionSteps progress={progressOf(stage)} />
        {/if}
      </div>
    {/if}
  </div>
</section>

{#if hostKeyOpen}
  <HostKeyModal {device} {api} onclose={() => (dismissed = stage)} onreconnect={reconnect} />
{/if}

<style>
  .session {
    display: flex;
    flex-direction: column;
    block-size: 100%;
    min-block-size: 0;
    background: var(--bg);
    color: var(--text);
  }

  .session[hidden] {
    display: none;
  }

  /* Solid and above the display, never over it (MAT-008). */
  .toolbar {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-bottom: var(--border-thin) solid var(--border);
    background: var(--surface);
  }

  .device {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .notice {
    font-size: var(--text-sm);
  }

  .stage {
    position: relative;
    flex: 1;
    min-block-size: 0;
  }

  /* The pane's size comes from the layout only, never from the display inside it, so a resize cannot loop (D-7). */
  .pane {
    position: absolute;
    inset: 0;
    overflow: hidden;
    outline: none;
  }

  .pane:focus-visible {
    box-shadow: inset 0 0 0 var(--border-focus) var(--accent);
  }

  /* The remote display: never animated, transformed, filtered or covered by a material (MAT-008). */
  .display {
    position: absolute;
    inset: 0;
  }

  /* Shown only while no display is: before ready and after the end (D-7). */
  .panel {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: var(--space-4);
    padding: var(--space-6);
    background: var(--bg);
  }
</style>
