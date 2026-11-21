<script lang="ts">
  import { tick } from 'svelte';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import { keyAction, moved } from './keys';
  import type { ThreadsStore } from './store.svelte';
  import { orderedThreads, threadState } from './state';
  import { narrowed, type Program } from './thread';
  import ThreadList from './view/ThreadList.svelte';
  import ThreadView from './view/ThreadView.svelte';

  interface Props {
    store: ThreadsStore;
    /** The time of now; a test gives its own. */
    now?: () => Date;
  }

  const { store, now = () => new Date() }: Props = $props();
  let program = $state<Program | null>(null);
  let search = $state('');
  let toolsOpen = $state(false);
  /** The group "Earlier" is folded at first (THR-056); a search shows what it holds. */
  let earlierChosen = $state(false);
  const earlierOpen = $derived(earlierChosen || search.trim() !== '');
  let field = $state<HTMLInputElement>();
  let screen = $state<HTMLElement>();
  const shown = $derived(narrowed(store.rows ?? [], program, search));
  /** The states are of the time of the last read of the list; a test gives its own time. */
  const moment = $derived.by(() => {
    void store.rows;
    return now();
  });
  /** The threads in the order of the list, for the keys that move by a place. */
  const inOrder = $derived(orderedThreads(shown, moment, store.seen, { earlierOpen, chosen: store.chosen }).flatMap((entry) => (entry.kind === 'device' ? [entry.device.row] : [])));

  async function choose(id: string): Promise<void> {
    await store.open(id);
    await tick();
    screen?.querySelector<HTMLElement>(`[data-thread="${CSS.escape(id)}"]`)?.scrollIntoView({ block: 'nearest' });
  }

  function typing(target: EventTarget | null): boolean {
    return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || (target instanceof HTMLElement && target.isContentEditable);
  }

  /** The keys of the page (THR-016). The rule is `keyAction`; this does what it names. */
  function onkeydown(event: KeyboardEvent): void {
    const action = keyAction({ key: event.key, mod: event.metaKey || event.ctrlKey, shift: event.shiftKey, alt: event.altKey, typing: typing(event.target) });
    if (action === null) return;
    const at = inOrder.findIndex((row) => row.id === store.chosen);
    switch (action.kind) {
      case 'move': {
        const next = inOrder[moved(at, action.by, inOrder.length)];
        if (next !== undefined && next.id !== store.chosen) void choose(next.id);
        break;
      }
      case 'jump': {
        const next = inOrder[action.place];
        if (next !== undefined) void choose(next.id);
        break;
      }
      case 'search':
        field?.focus();
        break;
      case 'leave':
        if (event.target instanceof HTMLElement) event.target.blur();
        break;
      case 'tools':
        toolsOpen = !toolsOpen;
        break;
      case 'find':
      case 'help':
        // The web view's own find, and the line of keys at the foot of the thread, serve these in stage 1a.
        return;
    }
    event.preventDefault();
  }
</script>

<svelte:window {onkeydown} />

<!-- The page Threads (THR-011 to THR-017): the list beside one thread. It reads the session files of this computer
     through the desktop shell; it sends nothing to the hub and starts no program. -->
<section class="threads" bind:this={screen}>
  <PageHeader title="Threads" detail="The sessions of Claude Code and Codex on this computer. Read only. The states are read from the session files." />
  {#if store.rows === null && store.listProblem !== null}
    <ErrorState message={store.listProblem} onRetry={() => void store.loadList()} />
  {:else if store.rows === null}
    <LoadingState label="Loading threads" rows={8} />
  {:else if store.rows.length === 0}
    <EmptyState title="No threads yet" line="When you use Claude Code or Codex on this computer, its sessions show here." />
  {:else}
    <div class="columns">
      <ThreadList
        rows={shown}
        chosen={store.chosen}
        {program}
        {search}
        now={moment}
        seen={store.seen}
        {earlierOpen}
        onToggleEarlier={() => {
          earlierChosen = !earlierChosen;
        }}
        bind:field
        onChoose={(id) => void choose(id)}
        onProgram={(next) => {
          program = next;
        }}
        onSearch={(next) => {
          search = next;
        }}
      />
      <div class="thread" aria-busy={store.loadingThread}>
        {#if store.threadProblem !== null}
          <ErrorState message={store.threadProblem} onRetry={() => (store.chosen === null ? undefined : void store.open(store.chosen))} />
        {:else if store.thread !== null && store.thread.id === store.chosen}
          <ThreadView thread={store.thread} {toolsOpen} state={threadState(store.thread, moment, store.seen)} />
        {:else if store.chosen !== null}
          <LoadingState label="Loading the thread" rows={8} />
        {:else}
          <EmptyState title="No thread is open" line="Select a thread in the list, or press the Down key." />
        {/if}
      </div>
    </div>
  {/if}
</section>

<style>
  .threads {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-inline-size: 0;
  }

  .columns {
    display: grid;
    grid-template-columns: calc(var(--space-16) * 4.5) minmax(0, 1fr);
    gap: var(--space-4);
    align-items: start;
  }

  .thread {
    min-inline-size: 0;
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-lg);
    background: var(--surface-base);
  }

  /* In a narrow window the list stands over the thread. */
  @media (width <= 60rem) {
    .columns {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
