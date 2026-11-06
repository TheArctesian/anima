<script lang="ts">
  import type { HubError, HubResult } from '$lib/hub/error';
  import FormButton from '../../FormButton.svelte';
  import type { WebInterfaceSuggestion } from '../../hub-pending';

  interface Props {
    suggestions: readonly WebInterfaceSuggestion[];
    onAccept: (checkId: string) => Promise<HubResult<unknown>>;
    onDismiss: (checkId: string) => Promise<HubResult<unknown>>;
  }

  const { suggestions, onAccept, onDismiss }: Props = $props();
  let working = $state<string | null>(null);
  let failure = $state<HubError | null>(null);

  async function act(checkId: string, call: (checkId: string) => Promise<HubResult<unknown>>): Promise<void> {
    if (working !== null) return;
    working = checkId;
    failure = null;
    const result = await call(checkId);
    working = null;
    if (!result.ok) failure = result.error;
  }
</script>

{#if suggestions.length > 0}
  <section aria-labelledby="web-suggestions-heading">
    <h5 id="web-suggestions-heading">Suggested from your HTTP checks</h5>
    {#if failure !== null}
      <p class="error" role="alert">{failure.message}</p>
    {/if}
    <ul>
      {#each suggestions as suggestion (suggestion.checkId)}
        <li>
          <span class="text"><strong>{suggestion.label}</strong> <span class="url">{suggestion.url}</span></span>
          <FormButton aria-label="Add {suggestion.label}" aria-disabled={working !== null} onclick={() => act(suggestion.checkId, onAccept)}>
            Add
          </FormButton>
          <FormButton aria-label="Dismiss {suggestion.label}" aria-disabled={working !== null} onclick={() => act(suggestion.checkId, onDismiss)}>
            Dismiss
          </FormButton>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  h5 {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
    font-weight: normal;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .text {
    flex: 1;
    min-inline-size: 0;
    overflow-wrap: anywhere;
  }

  .url {
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .error {
    margin: 0;
    color: var(--status-down);
  }
</style>
