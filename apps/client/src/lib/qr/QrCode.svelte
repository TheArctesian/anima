<script lang="ts">
  import { qrDrawing } from './qr';

  interface Props {
    /** What the code says. It may be a secret (an invite, a TOTP URI): it is never logged or put in an attribute. */
    text: string;
    /** What a screen reader says in place of the code; it must not repeat `text`. */
    label: string;
    /** Edge length as a CSS length; the default is a token. */
    size?: string;
  }

  const { text, label, size = 'var(--qr-size)' }: Props = $props();

  const drawing = $derived(qrDrawing(text));
</script>

<!-- Dark modules on a light tile in both themes, so a phone can read it. -->
{#if drawing}
<svg
  class="qr"
  style:inline-size={size}
  style:block-size={size}
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
  .qr {
    --qr-size: 12rem;
    display: block;
  }

  .tile {
    fill: var(--graphite-light-raised);
  }

  .modules {
    fill: var(--graphite-light-text);
  }
</style>
