<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';

  interface Props {
    /** Why the page is not shown: fixed text, never text from the hub or the page (R-035, LOG-007). */
    reason: string;
    /** The shell's short code, shown for a report; absent for a Down device. */
    code?: string;
    /** Absent when reloading cannot help: the interface itself cannot be opened. */
    onreload?: () => void;
  }

  const { reason, code, onreload }: Props = $props();
</script>

<!-- WEB-010: the reason and a Reload button, as EndedPanel. -->
<section class="down" role="status" tabindex="-1">
  <p class="reason">
    <Icon name="nf-md-lan_disconnect" />
    <span data-reason>{reason}</span>
  </p>
  {#if code !== undefined}
    <p class="code">Code: <span data-code>{code}</span></p>
  {/if}
  {#if onreload !== undefined}
    <button type="button" onclick={onreload} {@attach press()}>
      <Icon name="nf-md-refresh" />
      <span>Reload</span>
    </button>
  {/if}
</section>

<style>
  .down {
    /* Room for the focus ring when the page takes focus. */
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3);
    color: var(--text);
  }

  .down:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  p {
    margin: 0;
  }

  .reason {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-md);
    line-height: var(--leading-normal);
  }

  .code {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  [data-code] {
    font-family: var(--font-mono);
  }

  button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--accent);
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--bg);
    font: inherit;
    transition: background-color var(--duration-press) var(--ease-enter);
  }

  button:active {
    background: var(--accent-deep);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
