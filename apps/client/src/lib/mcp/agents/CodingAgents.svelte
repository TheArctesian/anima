<script lang="ts">
  import { onMount } from 'svelte';
  import { desktopAgentsShell, type AgentsShell } from '$lib/desktop/mcp/agents-shell';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { present, rowAction, stateTone, stateWords, toSetUp, touched, type AgentRow } from './agents';

  interface Props {
    /** The shell's commands; tests pass their own. */
    shell?: AgentsShell;
  }

  const { shell = desktopAgentsShell }: Props = $props();

  let rows = $state<readonly AgentRow[] | null>(null);
  let problem = $state<string | null>(null);
  let busy = $state(false);
  /** The change that waits for the person's yes, with the rows it touches (MCP-029). */
  let asking = $state<{ uninstall: boolean; rows: readonly AgentRow[] } | null>(null);
  const shown = $derived(rows === null ? [] : present(rows));
  const all = $derived(toSetUp(shown));

  async function run(job: () => Promise<readonly AgentRow[]>): Promise<readonly AgentRow[] | null> {
    busy = true;
    try {
      const answer = await job();
      problem = null;
      return answer;
    } catch (error) {
      problem = error instanceof Error ? error.message : 'The coding agents could not be read or changed.';
      return null;
    } finally {
      busy = false;
    }
  }

  async function load(): Promise<void> {
    rows = (await run(() => shell.read(false))) ?? rows;
  }

  // Once, when the part is shown: a read inside an effect would run again each time the rows change.
  onMount(() => {
    void load();
  });

  /** Reads what the change would touch, then waits for the yes: nothing is written before it. */
  async function ask(uninstall: boolean, ids: readonly string[]): Promise<void> {
    const listed = await run(() => shell.read(uninstall));
    if (listed === null) return;
    asking = { uninstall, rows: listed.filter((row) => ids.includes(row.id) && rowAction(row.state) === (uninstall ? 'uninstall' : 'install')) };
  }

  async function confirmed(): Promise<void> {
    const waiting = asking;
    asking = null;
    if (waiting === null || waiting.rows.length === 0) return;
    const done = await run(() => shell.change({ uninstall: waiting.uninstall, agents: waiting.rows.map((row) => row.id), force: false }));
    if (done !== null) rows = (rows ?? []).map((row) => done.find((each) => each.id === row.id) ?? row);
  }
</script>

<!-- MCP-039: each coding agent on this computer, whether it starts `anima mcp`, and the way to set it up or take it
     out. Before a change the part lists each file and each command; nothing is written before the person confirms. -->
<section class="agents" aria-labelledby="mcp-agents" aria-busy={busy}>
  <header>
    <h3 id="mcp-agents">Coding agents</h3>
    {#if all.length > 0}
      <FormButton size="sm" aria-disabled={busy} onclick={() => { if (!busy) void ask(false, all.map((row) => row.id)); }}>Set up all</FormButton>
    {/if}
  </header>
  <p class="note">Each agent starts <code>anima mcp</code> with the sign-in of the anima command. This changes no permission: the tool groups below apply to every agent.</p>
  {#if problem !== null}<p class="problem" role="alert">{problem}</p>{/if}
  {#if rows !== null && shown.length === 0}
    <p class="note">No coding agent that Anima.ing knows is on this computer.</p>
  {:else}
    <ul>
      {#each shown as row (row.id)}
        {@const action = rowAction(row.state)}
        <li>
          <span class="name">{row.name}</span>
          <span class="state {stateTone(row.state)}"><span class="dot" aria-hidden="true"></span>{stateWords(row.state)}</span>
          <span class="control">
            {#if action !== null}
              <FormButton size="sm" tone={action === 'install' ? 'secondary' : 'ghost'} aria-disabled={busy} aria-label="{action === 'install' ? 'Set up' : 'Remove from'} {row.name}" onclick={() => { if (!busy) void ask(action === 'uninstall', [row.id]); }}>
                {action === 'install' ? 'Set up' : 'Remove'}
              </FormButton>
            {/if}
          </span>
          {#if row.problem !== null}<p class="problem" role="alert">Not changed: {row.problem}.</p>{/if}
        </li>
      {/each}
    </ul>
  {/if}
  {#if asking !== null}
    <div class="asking" role="group" aria-label="What will change">
      {#if asking.rows.length === 0}
        <p>Nothing to change.</p>
        <FormButton size="sm" tone="ghost" onclick={() => { asking = null; }}>Close</FormButton>
      {:else}
        <p class="question">{asking.uninstall ? 'Take anima out of' : 'Set up'} {asking.rows.length === 1 ? asking.rows[0]?.name : `${String(asking.rows.length)} agents`}?</p>
        <ul class="steps">
          {#each asking.rows as row (row.id)}
            <li><span class="name">{row.name}</span> <span class="step">{touched(row)}</span></li>
          {/each}
        </ul>
        <p class="note">Only the entry "anima" changes. No token and no key is written.</p>
        <div class="buttons">
          <FormButton size="sm" tone="primary" onclick={() => void confirmed()}>{asking.uninstall ? 'Remove' : 'Set up'}</FormButton>
          <FormButton size="sm" tone="ghost" onclick={() => { asking = null; }}>Cancel</FormButton>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .agents {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-inline-size: 0;
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  h3 {
    flex: 1;
    margin: 0;
    color: var(--text);
    font-size: var(--text-md);
    font-weight: var(--weight-medium);
  }

  p {
    margin: 0;
  }

  .note {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .problem {
    color: var(--status-down);
    font-size: var(--text-sm);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .agents > ul > li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr) auto;
    column-gap: var(--space-3);
    align-items: center;
    min-block-size: var(--control-md);
    padding: var(--space-1);
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  .agents > ul > li > .problem {
    grid-column: 1 / -1;
  }

  .name {
    color: var(--text);
    font-weight: var(--weight-medium);
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  /* The dot repeats the words beside it; colour is never the only sign (MAT-005). */
  .dot {
    flex: none;
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--status-unknown);
  }

  .up .dot {
    background: var(--status-up);
  }

  .warning .dot {
    background: var(--status-warning);
  }

  .down .dot {
    background: var(--status-down);
  }

  .control {
    display: inline-flex;
    justify-content: end;
  }

  .asking {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  .question {
    color: var(--text);
    font-weight: var(--weight-medium);
  }

  .steps {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  /* A path is machine text and can be long: it wraps at any character, so the whole of it shows. */
  .step {
    color: var(--text-secondary);
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }

  .buttons {
    display: flex;
    gap: var(--space-2);
  }
</style>
