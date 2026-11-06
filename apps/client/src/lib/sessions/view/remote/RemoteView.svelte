<script lang="ts">
  import { untrack } from 'svelte';
  import type { RemoteProtocol, SessionsApi } from '../../api';
  import type { SessionEnding } from '../../ending';
  import { receiveClipboard } from '../../tunnel/guacamole/input';
  import type { GuacamoleState, SessionTunnel } from '../../tunnel/guacamole/tunnel';
  import { displayInput, type Size } from '../display';
  import EndedPanel from '../EndedPanel.svelte';
  import { keyRoute, pasteText, sendCombination, showDisplay } from '../guacamole/input';
  import { KEY_COMBINATIONS, keyFilter, type KeyCombination, type KeyFilter } from '../guacamole/keys';
  import KeysMenu from '../menu/KeysMenu.svelte';
  import Notice from '../pane/Notice.svelte';
  import { SessionConnection, progressOf } from '../session.svelte';
  import type { RemoteStart } from '../start';
  import ConnectionSteps from '../steps/ConnectionSteps.svelte';
  import { clipboardRules, COPY_FAILED, NO_CLIPBOARD, PASTE_FAILED, PASTE_REFUSED } from './clipboard';
  import { displayScale } from './fit';
  import { remotePane, type RemotePane } from './pane';

  type Client = SessionTunnel['client'];

  interface Props {
    device: { readonly id: string; readonly name: string };
    protocol: RemoteProtocol;
    /** Starts a session on `device` with a new ticket; see `sessionStart(...).remote(protocol)` in `../start`. */
    start: RemoteStart;
    api: Pick<SessionsApi, 'endSession'>;
    /** The pane is on screen (its tab is selected). */
    visible: boolean;
    /** The pane should take the keyboard (its tab is selected and the app has focus). */
    focused: boolean;
    /** Hears the final ending of each attempt (after `endSession` answered, D-4). */
    onended?: (ending: SessionEnding) => void;
    /** Hears an attempt that ended because the device's RDP certificate changed (close 4003, K4 §3, §5). */
    oncertificatechanged?: () => void;
    /**
     * Hears the client of each opened tunnel before the gateway's first frame, so the gateway's `required` prompt
     * can be set on `client.onrequired` (GW-015); the function it returns runs when that tunnel goes.
     */
    onclient?: (client: Client) => (() => void) | undefined;
  }

  const { device, protocol, start, api, visible, focused, onended, oncertificatechanged, onclient }: Props = $props();
  /** Fixed text for a `CONFLICT` refusal of the start (K4 §1); never the hub's own message (LOG-007). */
  const CONFLICT_TEXT: Readonly<Record<RemoteProtocol, string | undefined>> = {
    RDP: "Could not open the session. The device's RDP certificate is not accepted yet: run the connection test first.",
    VNC: undefined,
  };

  const connection = new SessionConnection<SessionTunnel, GuacamoleState>(
    (display, events) => start(display, events),
    { endSession: (id) => api.endSession(id) },
    (ending) => {
      onended?.(ending);
      if (connection.stage.kind === 'ended' && connection.stage.certificateChanged) oncertificatechanged?.();
    },
    untrack(() => CONFLICT_TEXT[protocol]),
  );
  let pane: RemotePane | undefined;
  /** The remote size the open tunnel started with, in device pixels. */
  let startedAt: Size = { width: 0, height: 0 };
  let paneElement: HTMLElement | undefined = $state();
  /** The pane's size in CSS px; read by the display scale, so it is state. */
  let paneSize: Size | undefined = $state();
  /** The remote display's own size, as the gateway last set it. */
  let remote: Size = $state({ width: 0, height: 0 });
  /** What the device holds and which keys reach it, for the open tunnel; the paste reads it too. */
  let filter: KeyFilter = keyFilter({ paste: false });
  let notice: string | undefined = $state();
  /** Whether the keyboard focus is inside the pane, and the window has it (D-8). */
  let paneHasFocus = $state(false);

  const stage = $derived(connection.stage);
  const client = $derived(connection.tunnel?.client);
  const ready = $derived(connection.ready);
  const rules = $derived(stage.kind === 'open' ? clipboardRules(stage.policy) : NO_CLIPBOARD);
  /** The parent gives this pane the keyboard; the pane then takes the focus. */
  const wanted = $derived(visible && focused && ready);
  /** Keys are sent only while the focus is really in the pane, so no key is held when it leaves (D-8). */
  const bound = $derived(wanted && paneHasFocus);
  const scale = $derived(displayScale(protocol, remote, paneSize ?? remote, window.devicePixelRatio));

  function connect(css: Size) {
    const display = displayInput(css, window.devicePixelRatio, Intl.DateTimeFormat().resolvedOptions().timeZone);
    startedAt = { width: display.widthPx, height: display.heightPx };
    void connection.connect(display);
  }

  function reconnect() {
    notice = undefined;
    connect(paneSize ?? { width: 0, height: 0 });
  }

  function copyToSystem(text: string) {
    navigator.clipboard.writeText(text).catch(() => {
      notice = COPY_FAILED;
    });
  }

  /** The operator's paste shortcut (Command+V, or Ctrl+V and the others the filter leaves to the browser). */
  function paste(event: ClipboardEvent) {
    if (client === undefined || !ready) return;
    event.preventDefault();
    if (!rules.paste) notice = PASTE_REFUSED;
    else pasteText(client, event.clipboardData?.getData('text/plain') ?? '', filter.held);
  }

  function sendKeys(combination: KeyCombination) {
    if (client !== undefined) sendCombination(client, combination);
    paneElement?.focus({ preventScroll: true });
  }

  function pasteFromClipboard() {
    const target = client;
    if (target === undefined || !rules.paste) return;
    navigator.clipboard.readText().then(
      (text) => {
        if (target === client && rules.paste) pasteText(target, text, filter.held);
        paneElement?.focus({ preventScroll: true });
      },
      () => {
        notice = PASTE_FAILED;
      },
    );
  }

  /** The pane's keyboard, focus and size: the first measure starts the session, later ones resize it (D-5). */
  function attachPane(element: HTMLElement) {
    paneElement = element;
    const attached = remotePane(element, {
      focus: (inside) => {
        // Chromium fires `focusout` while Svelte removes the pane, inside its block effect, where a tracked state
        // write is forbidden (`state_unsafe_mutation`); this handler reads nothing, so the write is untracked.
        untrack(() => {
          paneHasFocus = inside;
        });
      },
      measured: (css) => {
        const first = paneSize === undefined;
        paneSize = css;
        if (first) connect(css);
      },
      timer: {
        set: (callback, ms) => setTimeout(callback, ms),
        clear: (handle) => {
          clearTimeout(handle as ReturnType<typeof setTimeout>);
        },
      },
    });
    pane = attached;
    return () => {
      attached.stop();
      pane = undefined;
    };
  }

  // Each open tunnel gets a key filter for its policy, its clipboard and display hooks and, for RDP, a size sender.
  $effect(() => {
    if (client === undefined) return;
    const target = client;
    const allowed = untrack(() => rules);
    filter = keyFilter({ paste: allowed.paste });
    const display = target.getDisplay();
    remote = { width: display.getWidth(), height: display.getHeight() };
    display.onresize = (width, height) => {
      remote = { width, height };
    };
    // Text the gateway sends while copy is off, or while the pane is not bound, is dropped: a background session
    // never writes the system clipboard (RA-033, D-8).
    receiveClipboard(target, (text) => {
      if (allowed.copy && bound) copyToSystem(text);
    });
    const gone = untrack(() => onclient?.(target));
    const sizes = protocol === 'RDP' ? pane?.sendSizes(target, startedAt) : undefined;
    return () => {
      sizes?.();
      gone?.();
      display.onresize = null;
      target.onclipboard = null;
    };
  });

  $effect(() => {
    client?.getDisplay().scale(scale);
  });

  $effect(() => {
    if (wanted) paneElement?.focus({ preventScroll: true });
  });

  // The keyboard is bound only while the pane is visible, focused and ready and holds the DOM focus; never to the
  // document (D-8). Unbinding releases every held key on the device.
  $effect(() => {
    if (!bound || client === undefined || pane === undefined) return;
    const keyboard = pane.keyboard;
    keyboard.bind(keyRoute(client, filter));
    return () => {
      keyboard.unbind();
    };
  });

  $effect(() => () => {
    connection.close();
  });
</script>

<section class="session" hidden={!visible} aria-label="{protocol} session on {device.name}">
  <div class="toolbar">
    <KeysMenu disabled={!ready} entries={KEY_COMBINATIONS} onkeys={sendKeys} onpaste={pasteFromClipboard} pasteOff={rules.pasteOff} />
    <span class="device">{device.name}</span>
    {#if ready && rules.notice !== undefined}<span class="policy" data-policy title={rules.notice}>{rules.notice}</span>{/if}
    <!-- Notices lie over the toolbar row, which has a fixed height, so the display never moves for them (MOT-007). -->
    {#if notice !== undefined}
      <div class="cover">
        <Notice text={notice} ondismiss={() => (notice = undefined)} />
      </div>
    {/if}
  </div>
  <div class="stage">
    <!-- The keyboard listens here only; menus, panels and modals are outside it (D-8). -->
    <!-- Focusable because the remote desktop takes the keyboard as a text field does (D-8). -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="pane"
      role="application"
      aria-label="Remote display of {device.name}"
      tabindex="0"
      onpaste={paste}
      {@attach attachPane}
    >
      {#if ready && client !== undefined}
        <div class="display" data-display {@attach showDisplay(client)}></div>
      {/if}
    </div>
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

  /* Solid and above the display, never over it (MAT-008); a fixed height, so nothing in it moves the display (MOT-007). */
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

  /* The bar never sets the pane's width: the name and the policy give way first (UI-009). */
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

  .stage {
    position: relative;
    flex: 1;
    min-block-size: 0;
  }

  /* The pane's size comes from the layout only, never from the display inside it, so a resize cannot loop (D-5). */
  .pane {
    position: absolute;
    inset: 0;
    overflow: hidden;
    outline: none;
  }

  /* Drawn after the display, so a display that fills the pane cannot cover the ring (UI-009, UI-012). */
  .pane:focus-visible::after {
    content: '';
    position: absolute;
    inset: 0;
    pointer-events: none;
    box-shadow: inset 0 0 0 var(--border-focus) var(--accent);
  }

  /*
   * The remote display: never animated, filtered or covered by a material (MAT-008). The library puts each layer at
   * z-index -1, so the display makes its own stacking context; without it the layers paint behind the session.
   */
  .display {
    position: absolute;
    inset: 0;
    isolation: isolate;
  }

  /* Shown only while no display is: before ready and after the end. A narrow pane scrolls it, never the page (UI-009). */
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
