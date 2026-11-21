<script lang="ts">
  import Field from '$lib/forms/field/Field.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import Select from '$lib/forms/input/Select.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import { EFFORTS, MODE_WORDS, startOf, type NewThreadForm } from '../live/drive';
  import type { CallResult } from '../live/live.svelte';
  import type { Action, Effort, Mode } from '../live/run';
  import type { Program } from '../thread';

  interface Props {
    open: boolean;
    onStart: (action: Action) => Promise<CallResult>;
    /** The thread started: its ID. */
    onStarted: (thread: string) => void;
    onClose: () => void;
  }

  const { open, onStart, onStarted, onClose }: Props = $props();
  let program = $state<Program>('claude_code');
  let folder = $state('');
  let mode = $state<Mode>('full_access');
  let model = $state('');
  let effort = $state<Effort | ''>('');
  let prompt = $state('');
  let busy = $state(false);
  let problem = $state<string | null>(null);
  const MODES = Object.keys(MODE_WORDS) as Mode[];

  async function start(): Promise<void> {
    if (busy) return;
    const form: NewThreadForm = { program, folder, mode, model, effort, prompt };
    const checked = startOf(form);
    if (!checked.ok) {
      problem = checked.words;
      return;
    }
    busy = true;
    problem = null;
    const result = await onStart(checked.action);
    busy = false;
    if (!result.ok) {
      problem = result.words;
      return;
    }
    if (result.outcome.outcome === 'started') onStarted(result.outcome.thread);
    prompt = '';
    onClose();
  }
</script>

<!-- A new thread (THR-020): the program, the folder of this computer, the mode of approvals (full access is the
     default) and the first prompt. The runner checks each value again. -->
<Modal {open} title="New thread" {onClose} preset="md" dismissible>
  <form
    class="form"
    onsubmit={(event) => {
      event.preventDefault();
      void start();
    }}
  >
    <Field label="Program">
      <Select bind:value={program}>
        <option value="claude_code">Claude Code</option>
        <option value="codex">Codex</option>
      </Select>
    </Field>
    <Field label="Folder" constraint="An absolute folder of this computer, for example /Users/you/project">
      <Input bind:value={folder} mono placeholder="/Users/you/project" autocomplete="off" spellcheck={false} />
    </Field>
    <Field label="Approvals">
      <Select bind:value={mode}>
        {#each MODES as each (each)}<option value={each}>{MODE_WORDS[each]}</option>{/each}
      </Select>
    </Field>
    <Field label="Model" constraint="Empty for the default model of the program">
      <Input bind:value={model} mono autocomplete="off" spellcheck={false} />
    </Field>
    <Field label="Effort">
      <Select bind:value={effort}>
        <option value="">Default</option>
        {#each EFFORTS as each (each)}<option value={each}>{each}</option>{/each}
      </Select>
    </Field>
    <Field label="First prompt">
      <textarea class="prompt" bind:value={prompt} rows="4" aria-label="First prompt"></textarea>
    </Field>
    {#if problem !== null}<p class="problem" role="alert">{problem}</p>{/if}
    <div class="acts">
      <FormButton type="button" onclick={onClose}>Cancel</FormButton>
      <FormButton type="submit" tone="primary" busy={busy}>Start</FormButton>
    </div>
  </form>
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .prompt {
    inline-size: 100%;
    padding: var(--space-2);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text-primary);
    font: inherit;
    resize: vertical;
  }

  .acts {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .problem {
    margin: 0;
    color: var(--status-down);
    font-size: var(--text-sm);
  }
</style>
