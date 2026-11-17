<script lang="ts">
  import { PHONE_KEYS, type Latch, type Modifier } from './keys';

  interface Props {
    /** No session to send to (not ready, or ended). */
    disabled: boolean;
    latch: Latch;
    ontoggle: (modifier: Modifier) => void;
    onkeys: (bytes: Uint8Array<ArrayBuffer>) => void;
  }

  const { disabled, latch, ontoggle, onkeys }: Props = $props();

  /** A key must not take the focus from the terminal, or the on-screen keyboard closes. */
  function keepFocus(event: MouseEvent) {
    event.preventDefault();
  }
</script>

<!-- The row over the on-screen keyboard (RA-050, MOB-020; mockup 8). It scrolls sideways when it does not fit. -->
<div class="phone-session-keys" role="toolbar" aria-label="Terminal keys">
  {#each PHONE_KEYS as key, index (index)}
    {#if key.kind === 'gap'}
      <span class="gap" aria-hidden="true"></span>
    {:else if key.kind === 'modifier'}
      <button type="button" {disabled} aria-pressed={latch[key.modifier]} onmousedown={keepFocus} onclick={() => ontoggle(key.modifier)}>{key.label}</button>
    {:else}
      <button type="button" {disabled} aria-label={key.name} onmousedown={keepFocus} onclick={() => onkeys(key.bytes)}>{key.label}</button>
    {/if}
  {/each}
</div>
