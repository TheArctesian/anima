<script lang="ts">
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import { useHub } from '$lib/hub/context';
  import { skeletonDelay } from '$lib/motion/loading/delay.svelte';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import BulkActions from './BulkActions.svelte';
  import { groupTools, needsConfirmation, type McpChange } from './permissions';
  import { desktopMcpShell, type McpShell } from '$lib/desktop/mcp/shell';
  import { McpPermissionsStore } from './store.svelte';
  import ToolSetGroup from './ToolSetGroup.svelte';

  // The Settings page draws the section and its heading; this is what goes under it. Desktop only: the permissions are a
  // file on this computer (MCP-023).
  interface Props {
    /** The shell's permission commands; tests pass their own. */
    shell?: McpShell;
    /** The clock of the loading thresholds; tests pass their own. */
    clock?: LoadingClock;
  }

  const { shell = desktopMcpShell, clock }: Props = $props();
  const hub = useHub();
  // The hub address is fixed for the life of the section: changing the hub signs out and reloads the client.
  // svelte-ignore state_referenced_locally
  const store = new McpPermissionsStore(shell, hub.address.origin);
  const groups = $derived(groupTools(store.tools));
  /** The change waiting for the operator's yes. */
  let confirming = $state<McpChange | null>(null);
  const loading = $derived(store.status === 'loading');
  // svelte-ignore state_referenced_locally
  const skeleton = skeletonDelay(() => loading, clock);

  $effect(() => {
    void store.load();
  });

  /** Allow-all turns change tools on, so it waits for a yes; every other change is saved at once (MCP-020). */
  function request(change: McpChange): void {
    if (needsConfirmation(change)) confirming = change;
    else void store.change(change);
  }

  async function confirmed(): Promise<{ ok: true; value: undefined }> {
    const change = confirming;
    confirming = null;
    if (change !== null) void store.change(change);
    return { ok: true, value: undefined };
  }
</script>

<div class="mcp">
  <div class="intro">
    <p>
      The MCP server lets a coding agent on this computer work with this hub. Start it with <code>anima mcp</code>. These permissions
      apply to every agent that uses it for this hub, and they are saved in a file on this computer.
    </p>
    <p>
      Allow lets the agent call the tool. Ask shows you a question before each call. Off hides the tool. Your hub sign-in or API key
      still limits everything: a key with read scope cannot change anything, whatever you allow here.
    </p>
  </div>

  {#if store.status === 'failed'}
    <ErrorState message={store.loadProblem} onRetry={() => void store.load()} />
  {:else}
    <div class="stack" aria-busy={loading}>
      {#if store.status === 'ready'}
        <div class="all">
          <span class="name">All tools</span>
          <BulkActions scope="all tools" onrequest={request} />
        </div>
        <div class="groups">
          {#each groups as group (group.toolset)}
            <ToolSetGroup {group} onrequest={request} />
          {/each}
        </div>
      {:else if skeleton.visible}
        <LoadingState label="Loading MCP permissions" rows={4} />
      {/if}
    </div>
  {/if}

  <!-- One line each, always mounted, so a result moves nothing below it (MOT-007). The bar marks the failure beside the words (UI-003). -->
  <p class="saved" role="status">{store.saved === '' ? '' : `Saved. ${store.saved}`}</p>
  <p class="problem" role="alert">{store.writeProblem === '' ? '' : `Not saved. ${store.writeProblem} The control went back to its earlier state.`}</p>
</div>

{#key confirming}
  <ConfirmModal
    open={confirming !== null}
    title="Allow all changes?"
    body="Coding agents may then change this hub without asking you: add, edit and delete devices, checks, releases and more. The hub token still limits what they can do."
    confirmLabel="Allow all changes"
    onConfirm={confirmed}
    onCancel={() => {
      confirming = null;
    }}
  />
{/key}

<style>
  .mcp {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
  }

  .intro {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  p {
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  code {
    font-family: var(--font-mono);
  }

  .stack,
  .groups {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .all {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .name {
    font-weight: var(--weight-bold);
  }

  .saved,
  .problem {
    min-block-size: calc(var(--text-sm) * var(--leading-tight));
  }

  .saved {
    color: var(--text-secondary);
  }

  .problem {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid transparent;
  }

  .problem:not(:empty) {
    border-inline-start-color: var(--status-down);
  }
</style>
