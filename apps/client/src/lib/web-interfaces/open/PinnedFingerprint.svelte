<script lang="ts">
  import { pinnedFingerprintToShow } from './pinned';

  interface Props {
    /** The element's id, so a control can name it as its description. */
    id: string;
    webInterface: { readonly url: string; readonly pinnedFingerprintSha256?: string | null };
  }

  const { id, webInterface }: Props = $props();
  const fingerprint = $derived(pinnedFingerprintToShow(webInterface));
</script>

<!-- WEB-011: in a browser the pin cannot be enforced, so it is shown for the operator to compare with the browser's certificate view. -->
{#if fingerprint !== null}
  <span class="pinned" {id}><span class="caption">Pinned certificate SHA-256</span><code>{fingerprint}</code></span>
{/if}

<style>
  .pinned {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
    /* The menu panel does not wrap its text; the pin does. */
    white-space: normal;
    max-inline-size: calc(var(--space-16) * 4);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  code {
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }
</style>
