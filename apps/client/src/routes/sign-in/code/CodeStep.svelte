<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import CodeBoxes from './CodeBoxes.svelte';

  interface Props {
    busy: boolean;
    oncomplete: (code: string) => void;
    /** Back to the user name and password. */
    onback: () => void;
    /** Sign in with no code: an account with no second factor (OD-2). */
    onskip: () => void;
  }

  const { busy, oncomplete, onback, onskip }: Props = $props();
</script>

<div class="step">
  <h2>Enter the code from your authenticator app</h2>
  <CodeBoxes disabled={busy} {oncomplete} />
  <p class="busy" role="status">{#if busy}<Icon name="nf-md-refresh" />Signing in{/if}</p>
  <div class="links">
    <button type="button" tabindex="0" disabled={busy} onclick={onback}>Back</button>
    <button type="button" tabindex="0" disabled={busy} onclick={onskip}>Sign in without a code</button>
  </div>
</div>

<style>
  .step {
    display: grid;
    gap: var(--space-4);
    justify-items: center;
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-medium);
    line-height: var(--leading-tight);
    text-align: center;
  }

  .busy {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    min-block-size: var(--space-4);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-4);
  }

  .links button {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: var(--text-sm);
    text-decoration: underline;
    cursor: pointer;
  }

  .links button:disabled {
    opacity: var(--opacity-disabled);
    cursor: default;
  }

  .links button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }
</style>
