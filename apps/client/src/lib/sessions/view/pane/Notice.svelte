<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';

  interface Props {
    /** One fixed sentence; it never holds text from the device (LOG-007). */
    text: string;
    ondismiss: () => void;
  }

  const { text, ondismiss }: Props = $props();
</script>

<span class="notice" role="status" title={text}>{text}</span>
<button type="button" class="dismiss" aria-label="Dismiss the message" onclick={ondismiss} {@attach press()}>
  <Icon name="nf-md-close" size="sm" />
</button>

<style>
  /* One line, whatever the pane's width; the full text is the tooltip. */
  .notice {
    flex: 1;
    min-inline-size: 0;
    overflow: hidden;
    font-size: var(--text-sm);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dismiss {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    min-inline-size: var(--target-min-desktop);
    min-block-size: var(--target-min-desktop);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
  }

  .dismiss:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }
</style>
