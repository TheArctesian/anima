<script lang="ts">
  import { tick } from 'svelte';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import { fillRoom } from '$lib/scroll/room';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import { keyAction, moved, type KeyAction } from './keys';
  import type { ThreadsStore } from './store.svelte';
  import { orderedThreads, threadState } from './state';
  import { narrowed, type Program } from './thread';
  import ThreadList from './view/ThreadList.svelte';
  import ThreadView from './view/ThreadView.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { CallResult, LiveThreads } from './live/live.svelte';
  import type { Answer } from './live/run';
  import LimitDialog from './view/LimitDialog.svelte';
  import LiveView from './view/LiveView.svelte';
  import NewThread from './view/NewThread.svelte';

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
  let starting = $state(false);
  const live = $derived(store.live);
  const linkWords = $derived(live === null ? null : live.linked === false ? (live.problem ?? 'The thread runner of this computer is not running. Start the Anima daemon to start threads.') : live.holder);
  let screen = $state<HTMLElement>();
  const shown = $derived(narrowed(store.allRows ?? [], program, search));
  /** The states are of the time of the last read of the list; a test gives its own time. */
  const moment = $derived.by(() => {
    void store.allRows;
    return now();
  });
  /** The threads in the order of the list, for the keys that move by a place. */
  const inOrder = $derived(orderedThreads(shown, moment, store.seen, { earlierOpen, chosen: store.chosen }).flatMap((entry) => (entry.kind === 'device' ? [entry.device.row] : [])));

  /** The start that waited at the soft limit goes again: when it started, its thread opens and the form closes. */
  async function raise(threads: LiveThreads): Promise<CallResult> {
    const result = await threads.raiseLimit();
    if (result.ok && result.outcome.outcome === 'started') {
      starting = false;
      void choose(result.outcome.thread);
    }
    return result;
  }

  async function choose(id: string): Promise<void> {
    await store.open(id);
    await tick();
    screen?.querySelector<HTMLElement>(`[data-thread="${CSS.escape(id)}"]`)?.scrollIntoView({ block: 'nearest' });
  }

  function typing(target: EventTarget | null): boolean {
    return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || (target instanceof HTMLElement && target.isContentEditable);
  }

  /** The action of a key on the page. In the scroll area of the thread the arrow keys scroll it, not change the thread. */
  function pageAction(event: KeyboardEvent): KeyAction | null {
    const mod = event.metaKey || event.ctrlKey;
    const action = keyAction({ key: event.key, mod, shift: event.shiftKey, alt: event.altKey, typing: typing(event.target) });
    const inPane = event.target instanceof Element && event.target.closest('[data-own-keys]') !== null;
    return action?.kind === 'move' && !mod && inPane ? null : action;
  }

  /** The keys of the page (THR-016). The rule is `keyAction`; this does what it names. */
  function onkeydown(event: KeyboardEvent): void {
    const action = pageAction(event);
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
<section class="threads" bind:this={screen} {@attach fillRoom()}>
  <PageHeader title="Threads" detail="The sessions of Claude Code and Codex on this computer. Threads started here are driven on this computer only.">
    {#snippet actions()}
      {#if live !== null}<FormButton tone="primary" icon="carbon:add" onclick={() => (starting = true)} disabled={live.linked === false}>New thread</FormButton>{/if}
    {/snippet}
  </PageHeader>
  {#if linkWords !== null}<p class="link" role="status">{linkWords}</p>{/if}
  {#if store.rows === null && store.listProblem !== null}
    <ErrorState message={store.listProblem} onRetry={() => void store.loadList()} />
  {:else if store.rows === null}
    <LoadingState label="Loading threads" rows={8} />
  {:else if (store.allRows ?? []).length === 0}
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
        {:else if store.liveThread !== null && live !== null}
          {@const id = store.liveThread.row.id}
          <!-- Keyed by the thread, so that a draft of the composer never goes to a different thread. -->
          {#key id}
          <LiveView
            thread={store.liveThread}
            {toolsOpen}
            onSend={(text) => live.call({ call: 'send', thread: id, text })}
            onAnswer={(request: string, answer: Answer) => live.call({ call: 'answer', thread: id, request, answer })}
            onInterrupt={() => live.call({ call: 'interrupt', thread: id })}
            onStop={() => live.call({ call: 'stop', thread: id })}
          />
          {/key}
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
{#if live !== null}
  <NewThread
    open={starting}
    onStart={(action) => live.call(action)}
    onStarted={(id) => void choose(id)}
    onClose={() => {
      starting = false;
    }}
  />
  <LimitDialog open={live.overLimit !== null} onRaise={() => raise(live)} onKeep={() => live.keepLimit()} />
{/if}

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

  .link {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .thread {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-lg);
    background: var(--surface-base);
  }

  /* In a wide window the page fills the window and does not scroll: the list and the thread each scroll on their
     own, and the header of the thread, the search and the filter stay in view. */
  @media (width > 60rem) {
    .threads {
      block-size: var(--room, auto);
    }

    .columns {
      flex: 1;
      align-items: stretch;
      min-block-size: 0;
    }

    .columns > :global(*) {
      min-block-size: 0;
    }

    .thread {
      overflow: hidden;
    }
  }

  /* In a narrow window the list stands over the thread. */
  @media (width <= 60rem) {
    .columns {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
