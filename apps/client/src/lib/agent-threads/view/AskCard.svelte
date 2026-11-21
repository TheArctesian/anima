<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import { askTitle, chosenAnswer, DECLINED } from '../live/drive';
  import type { CallResult } from '../live/live.svelte';
  import type { Answer } from '../live/run';
  import type { OpenAsk } from '../live/timeline';

  interface Props {
    open: OpenAsk;
    /** The name of the program, as the head says it. */
    program: string;
    /** The name of the folder of the thread. */
    folder: string;
    onAnswer: (request: string, answer: Answer) => Promise<CallResult>;
  }

  const { open, program, folder, onAnswer }: Props = $props();
  const ask = $derived(open.ask);
  let busy = $state(false);
  let problem = $state<string | null>(null);
  let chosen = $state<Record<string, string[]>>({});
  const answers = $derived(chosenAnswer(ask, chosen));

  async function give(answer: Answer): Promise<void> {
    if (busy) return;
    busy = true;
    problem = null;
    const result = await onAnswer(open.request, answer);
    // After an answer that went, the card waits for the runner to close the request.
    if (!result.ok) {
      busy = false;
      problem = result.words;
    }
  }

  function pick(key: string, choice: string, many: boolean): void {
    const now = chosen[key] ?? [];
    const next = many ? (now.includes(choice) ? now.filter((each) => each !== choice) : [...now, choice]) : [choice];
    chosen = { ...chosen, [key]: next };
  }
</script>

<!-- A request of the program (THR-030, THR-032): its text is drawn as text. The answer goes to the program on this
     computer only; the request closes when the runner says that the program took it. -->
<div class="ask" role="group" aria-label="Request" data-request={open.request}>
  <b>{askTitle(program, ask)}</b>
  {#if ask.subject !== ''}<code class="subject">{ask.subject}</code>{/if}
  {#if ask.diff !== null}<pre class="diff">{ask.diff}</pre>{/if}
  {#each ask.questions as question (question.key)}
    <fieldset>
      <legend>{question.header ?? question.text}</legend>
      {#if question.header !== null}<p>{question.text}</p>{/if}
      <div class="choices">
        {#each question.choices as choice (choice)}
          <label>
            <input type={question.many ? 'checkbox' : 'radio'} name={`${open.request}-${question.key}`} checked={(chosen[question.key] ?? []).includes(choice)} onchange={() => pick(question.key, choice, question.many)} />
            {choice}
          </label>
        {/each}
      </div>
    </fieldset>
  {/each}
  {#if ask.kind !== 'question' && ask.sessionRules.length > 0}<span class="where">"Allow for this thread" allows: <code class="subject">{ask.sessionRules[0]}</code></span>{/if}
  <span class="where">In {folder}, on this computer. The answer goes to the program on this computer only.</span>
  {#if problem !== null}<p class="problem" role="alert">{problem}</p>{/if}
  <div class="acts">
    {#if ask.kind === 'question'}
      <FormButton size="sm" tone="primary" aria-disabled={busy || answers === null} onclick={() => (answers === null ? undefined : void give(answers))}>Send the answers</FormButton>
    {:else}
      <FormButton size="sm" tone="primary" aria-disabled={busy} onclick={() => void give({ answer: 'allow' })}>Allow once</FormButton>
      {#if ask.sessionRules.length > 0}
        <FormButton size="sm" aria-disabled={busy} onclick={() => void give({ answer: 'allow_for_session', rule: 0 })}>Allow for this thread</FormButton>
      {/if}
    {/if}
    <FormButton size="sm" aria-disabled={busy} onclick={() => void give({ answer: 'deny', message: DECLINED })}>Decline</FormButton>
  </div>
</div>

<style>
  .ask {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: var(--border-thin) solid var(--status-degraded);
    border-radius: var(--radius-lg);
    background: var(--status-degraded-tint);
  }

  .subject,
  .diff {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .diff {
    margin: 0;
    max-block-size: calc(var(--space-16) * 4);
    overflow: auto;
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    font-weight: var(--weight-medium);
  }

  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
  }

  .where {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .problem {
    margin: 0;
    color: var(--status-down);
    font-size: var(--text-sm);
  }
</style>
