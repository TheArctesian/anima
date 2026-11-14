<script lang="ts">
  import { useHub } from '$lib/hub/context';
  import { statusLabel } from '../nav';

  interface Props {
    /** The bar shows icons only: the dot stands in for the words, which stay as the tooltip and the accessible name. */
    iconOnly?: boolean;
  }

  const { iconOnly = false }: Props = $props();
  const hub = useHub();
  const connected = $derived(hub.status.current.connection === 'connected');
</script>

<!-- The hub connection at the foot of the bar, above Settings (NAV-002). -->
<p class="status" title={iconOnly ? statusLabel(hub.status.current) : undefined}>
  <span class="dot" class:connected aria-hidden="true"></span><span class="words">{statusLabel(hub.status.current)}</span>
</p>

<style>
  /* One line with a height set from tokens, so the font swap cannot move the divider (MOT-007). */
  .status {
    flex: none;
    margin: 0;
    padding: 0 var(--space-3);
    block-size: calc(var(--text-sm) * var(--leading-tight));
    line-height: var(--leading-tight);
    overflow: hidden;
    white-space: nowrap;
    color: var(--text);
    font-size: var(--text-sm);
  }

  /*
   * Connected is a filled dot and anything else a ring, so the state is a shape and not a colour alone (UI-003). The
   * outline keeps 3:1 against the bar; under forced colours the system colours take over.
   */
  .dot {
    display: none;
    flex: none;
    box-sizing: border-box;
    inline-size: var(--space-3);
    block-size: var(--space-3);
    border: var(--border-thin) solid var(--text);
    border-radius: var(--radius-pill);
  }

  .dot.connected {
    background: var(--status-up);
  }

  @media (forced-colors: active) {
    .dot {
      /* stylelint-disable-next-line scale-unlimited/declaration-strict-value -- forced colours use system colours, not tokens */
      border-color: CanvasText;
    }

    .dot.connected {
      /* stylelint-disable-next-line scale-unlimited/declaration-strict-value -- forced colours use system colours, not tokens */
      background: Highlight;
    }
  }

  /* Icons only: the dot shows and the words leave the layout but stay in the page (UI-015, MOT-006). */
  :global(:root[data-nav='icons']) .status {
    display: flex;
    justify-content: center;
    padding: 0;
  }

  :global(:root[data-nav='icons']) .dot {
    display: block;
  }

  :global(:root[data-nav='icons']) .words {
    position: absolute;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
