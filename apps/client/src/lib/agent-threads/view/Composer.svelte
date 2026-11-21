<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { CallResult } from '../live/live.svelte';
  import type { RunRow } from '../live/run';
  import { composerKey, MODE_WORDS } from '../live/drive';

  interface Props {
    row: RunRow;
    onSend: (text: string) => Promise<CallResult>;
    onInterrupt: () => Promise<CallResult>;
    onStop: () => Promise<CallResult>;
  }

  const { row, onSend, onInterrupt, onStop }: Props = $props();
  let text = $state('');
  let busy = $state(false);
  let problem = $state<string | null>(null);
  const running = $derived(row.state !== 'stopped');
  const working = $derived(row.state === 'working' || row.state === 'starting' || row.state === 'needs_you');

  async function run(call: () => Promise<CallResult>, clear: boolean): Promise<void> {
    if (busy) return;
    busy = true;
    problem = null;
    const result = await call();
    busy = false;
    if (!result.ok) problem = result.words;
    else if (clear) text = '';
  }

  function send(): void {
    if (text.trim() === '') return;
    const prompt = text;
    void run(() => onSend(prompt), true);
  }

  /** Enter sends; Shift+Enter makes a new line (THR-017). */
  function onkeydown(event: KeyboardEvent): void {
    if (composerKey({ key: event.key, shift: event.shiftKey, composing: event.isComposing }) !== 'send') return;
    event.preventDefault();
    send();
  }
</script>

<!-- The composer of a driven thread (THR-017): the prompt goes to the program on this computer only. A prompt sent
     while the agent works goes to the running turn. -->
<form
  class="composer"
  aria-label="Composer"
  onsubmit={(event) => {
    event.preventDefault();
    send();
  }}
>
  <textarea bind:value={text} {onkeydown} rows="3" disabled={!running} aria-label="Prompt" placeholder={running ? 'Ask for a change' : 'The program of this thread ended'}></textarea>
  {#if problem !== null}<p class="problem" role="alert">{problem}</p>{/if}
  <div class="acts">
    <span class="fact">{row.model ?? 'Default model'}</span>
    <span class="fact">{MODE_WORDS[row.mode]}</span>
    <span class="grow"></span>
    {#if running}
      <FormButton size="sm" tone="ghost" type="button" onclick={() => void run(onStop, false)} aria-disabled={busy}>End the program</FormButton>
      <FormButton size="sm" type="button" onclick={() => void run(onInterrupt, false)} aria-disabled={busy || !working} disabled={!working}>Stop</FormButton>
    {/if}
    <FormButton size="sm" tone="primary" type="submit" aria-disabled={busy || !running || text.trim() === ''} disabled={!running}>Send</FormButton>
  </div>
</form>

<style>
  .composer {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: var(--space-3) var(--space-4);
    padding: var(--space-3);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  textarea {
    inline-size: 100%;
    min-block-size: calc(var(--space-16) * 1.25);
    resize: vertical;
    border: 0;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
    line-height: var(--leading-normal);
  }

  textarea:focus-visible {
    outline: none;
  }

  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    align-items: center;
  }

  .fact {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .grow {
    flex: 1;
  }

  .problem {
    margin: 0;
    color: var(--status-down);
    font-size: var(--text-sm);
  }
</style>
