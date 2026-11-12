<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useActionFeedback } from '$lib/forms/feedback/feedback.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { hostKeyText, type OverviewDevice } from '../overview';

  interface Props {
    ssh: OverviewDevice['ssh'];
  }

  const { ssh }: Props = $props();
  const id = $props.id();
  let copy = $state<'idle' | 'copied' | 'failed'>('idle');
  const copied = useActionFeedback();

  async function copyFingerprint(fingerprint: string): Promise<void> {
    try {
      await copied.track(navigator.clipboard.writeText(fingerprint), () => true);
      copy = 'copied';
    } catch {
      // No clipboard, or the system refused it; the fingerprint stays on screen to select by hand.
      copy = 'failed';
    }
  }
</script>

<!-- D-10: the accepted SSH host key, its algorithm as a label and its fingerprint whole, or why there is none. -->
<section class="overview-card key-card" aria-labelledby="{id}-title">
  <h2 id="{id}-title" class="overview-card-title"><Icon name="nf-md-fingerprint" />SSH host key</h2>
  {#if ssh.hostKey != null}
    {@const key = ssh.hostKey}
    <span class="algorithm">{key.algorithm}</span>
    <p class="overview-key">{key.fingerprintSha256}</p>
    <div class="copy">
      <FormButton aria-label="Copy the host key fingerprint" feedback={copied.state} doneLabel="Copied" onclick={() => void copyFingerprint(key.fingerprintSha256)}>
        <span class="label"><Icon name="nf-md-content_copy" size="sm" />Copy</span>
      </FormButton>
      <!-- The live region holds only the outcome of Copy; it has its line before it speaks, so nothing moves (MOT-007).
           The button shows success itself, so "Copied." is for screen readers only. -->
      <span class="outcome" class:spoken={copy === 'copied'} role="status">{copy === 'copied' ? 'Copied.' : copy === 'failed' ? 'Could not copy. Select the fingerprint and copy it.' : ''}</span>
    </div>
  {:else}
    <p class="none">{hostKeyText(ssh)}</p>
  {/if}
</section>

<style>
  .algorithm {
    justify-self: start;
    padding: 0 var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    line-height: var(--leading-normal);
  }

  .copy {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .outcome,
  .none {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .none {
    margin: 0;
  }

  .spoken {
    position: absolute;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
