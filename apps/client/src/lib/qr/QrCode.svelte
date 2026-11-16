<script lang="ts">
  import { qrDrawing } from './qr';

  interface Props {
    /** What the code says. It may be a secret (an invite, a TOTP URI): it is never logged or put in an attribute. */
    text: string;
    /** What a screen reader says in place of the code; it must not repeat `text`. */
    label: string;
  }

  const { text, label }: Props = $props();

  const drawing = $derived(qrDrawing(text));
</script>

<!-- Dark modules on a light tile in both themes, so a phone can read it. -->
{#if drawing}
<svg
  class="qr"
  viewBox={`0 0 ${String(drawing.side)} ${String(drawing.side)}`}
  role="img"
  aria-label={label}
  shape-rendering="crispEdges"
>
  <rect width={drawing.side} height={drawing.side} class="tile" />
  <path d={drawing.path} class="modules" />
</svg>
{/if}

<style>
  /* 192 px: three of the largest space step. K5 has no size name for a code, so the length comes from a space token (TOK-003). */
  .qr {
    display: block;
    inline-size: calc(var(--space-16) * 3);
    block-size: calc(var(--space-16) * 3);
  }

  .tile {
    fill: var(--graphite-light-raised);
  }

  .modules {
    fill: var(--graphite-light-text);
  }
</style>
