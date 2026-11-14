<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import { useHub } from '$lib/hub/context';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import BulkActions from './BulkActions.svelte';
  import { allowQuestion } from './allowance';
  import { groupTools, needsConfirmation, TOOLSET_COUNT, type McpChange } from './permissions';
  import { desktopMcpShell, type McpShell } from '$lib/desktop/mcp/shell';
  import { McpPermissionsStore } from './store.svelte';
  import ToolSetGroup from './ToolSetGroup.svelte';

  // The Settings page draws the section and its heading; this is what goes under it. Desktop only: the permissions are a
  // file on this computer (MCP-023).
  interface Props {
    /** The shell's permission commands; tests pass their own. */
    shell?: McpShell;
  }

  const { shell = desktopMcpShell }: Props = $props();
  const hub = useHub();
  // The hub address is fixed for the life of the section: changing the hub signs out and reloads the client.
  // svelte-ignore state_referenced_locally
  const store = new McpPermissionsStore(shell, hub.address.origin);
  const groups = $derived(groupTools(store.tools));
  /** The "Allow all" waiting for the operator's yes. */
  let confirming = $state<Extract<McpChange, { type: 'bulk' }> | null>(null);
  const question = $derived(confirming === null ? null : allowQuestion(confirming, groups.map((group) => group.toolset)));

  $effect(() => {
    void store.load();
  });

  /** Allow-all turns change tools on, so it waits for a yes; every other change is saved at once (MCP-020). */
  function request(change: McpChange): void {
    if (needsConfirmation(change) && change.type === 'bulk') confirming = change;
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
      The MCP server (Model Context Protocol, a way for coding agents to use tools) lets a coding agent on this computer work with this hub.
      Start it with <code>anima mcp</code>. These permissions apply to every agent that uses it for this hub, and they are saved in a file on
      this computer.
    </p>
    <p>
      Allow lets an agent use a tool without asking you. Text that an agent reads, such as a device name or a log line, can steer it.
    </p>
    <p>
      Ask: the agent's app shows you each call to approve. If that app cannot ask, the call is refused. Off hides the tool. Your hub sign-in or
      API key still limits everything: a key with read scope cannot change anything, whatever you allow here.
    </p>
  </div>

  <!-- Loading, failure and the list fill one block of the same height at the first paint, so nothing below moves (SET-004, MOT-007). -->
  <div class="stack" aria-busy={store.status === 'loading'}>
    {#if store.status === 'ready'}
      <div class="listed">
        <div class="all">
          <span class="name">All tools</span>
          <BulkActions scope="all tools" onrequest={request} />
        </div>
        <div class="groups">
          {#each groups as group (group.toolset)}
            <ToolSetGroup {group} onrequest={request} />
          {/each}
        </div>
      </div>
    {:else}
      <div
        class="placeholder"
        class:hidden={store.status === 'failed'}
        role={store.status === 'failed' ? undefined : 'status'}
        aria-label={store.status === 'failed' ? undefined : 'Loading MCP permissions'}
        aria-hidden={store.status === 'failed'}
        inert={store.status === 'failed'}
      >
        <div class="all">
          <span class="name">All tools</span>
          <BulkActions scope="all tools" onrequest={request} blocked />
        </div>
        <!-- One closed group header each, at the header's own height. -->
        <div class="groups">
          {#each Array.from({ length: TOOLSET_COUNT }, (_, index) => index) as row (row)}<div class="row"></div>{/each}
        </div>
      </div>
      {#if store.status === 'failed'}
        <div class="failed"><ErrorState message={store.loadProblem} onRetry={() => void store.load()} /></div>
      {/if}
    {/if}
  </div>

  <!-- Each line has a reserved block, always mounted, so a result moves nothing below it (MOT-007). The bar marks the failure beside the words (UI-003). -->
  <p class="saved" role="status">{store.saved === '' ? '' : `Saved. ${store.saved}`}</p>
  <div class="problem" role="alert">
    {#if store.problem !== null}
      <span>Not saved: {store.problem.attempt}. {store.problem.reason} It went back to its earlier state.</span>
      <FormButton size="sm" onclick={() => { store.dismissProblem(); }}>Dismiss</FormButton>
    {/if}
  </div>
</div>

{#key confirming}
  <ConfirmModal
    open={question !== null}
    title={question?.title ?? ''}
    body={question?.body ?? ''}
    lines={question?.lines}
    confirmLabel={question?.confirmLabel ?? ''}
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

  .listed,
  .groups,
  .placeholder {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  /* The block's own list, the placeholders and the failure share one cell; the block is as tall as the tallest. */
  .stack {
    display: grid;
  }

  .stack > * {
    grid-area: 1 / 1;
    min-inline-size: 0;
  }

  .row {
    box-sizing: border-box;
    min-block-size: var(--space-10);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--surface-hover);
  }

  .hidden {
    visibility: hidden;
  }

  .failed {
    align-self: start;
  }

  .all {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .name {
    font-weight: var(--weight-medium);
  }

  /* Reserved: the text's own line, as the text uses it. The failure may take two lines and the button's height. */
  .saved {
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
    color: var(--text-secondary);
  }

  .problem {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
    min-block-size: max(var(--control-sm), calc(var(--text-sm) * var(--leading-normal) * 2));
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid transparent;
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .problem:not(:empty) {
    border-inline-start-color: var(--status-down);
  }
</style>
