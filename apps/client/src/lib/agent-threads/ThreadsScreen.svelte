<script lang="ts">
  import { tick } from 'svelte';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import { fillRoom } from '$lib/scroll/room';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import { keyAction, moved, type KeyAction } from './keys';
  import type { ThreadsStore } from './store.svelte';
  import { changedNow, narrowed, type Program } from './thread';
  import ThreadList from './ThreadList.svelte';
  import ThreadView from './ThreadView.svelte';

  interface Props {
    store: ThreadsStore;
    /** The time of now; a test gives its own. */
    now?: () => Date;
  }

  const { store, now = () => new Date() }: Props = $props();
  let program = $state<Program | null>(null);
  let search = $state('');
  let toolsOpen = $state(false);
  let field = $state<HTMLInputElement>();
  let screen = $state<HTMLElement>();
  const shown = $derived(narrowed(store.rows ?? [], program, search));
  const moment = $derived.by(() => {
    void store.rows;
    return now();
  });

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
    const at = shown.findIndex((row) => row.id === store.chosen);
    switch (action.kind) {
      case 'move': {
        const next = shown[moved(at, action.by, shown.length)];
        if (next !== undefined && next.id !== store.chosen) void choose(next.id);
        break;
      }
      case 'jump': {
        const next = shown[action.place];
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
  <PageHeader title="Threads" detail="The sessions of Claude Code and Codex on this computer. Read only." />
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
          <ThreadView thread={store.thread} {toolsOpen} changedNow={changedNow(store.thread, moment)} />
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
