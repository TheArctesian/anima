<script lang="ts">
  import Field from '$lib/forms/field/Field.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import ChangeDialog from '../forge/ChangeDialog.svelte';
  import type { ForgeApi } from '../forge/api';
  import type { Issue } from '../forge/types';

  interface Props {
    api: ForgeApi;
    repository: string;
    onDone: (issue: Issue) => void;
    onCancel: () => void;
  }

  const { api, repository, onDone, onCancel }: Props = $props();
  let title = $state('');
  let body = $state('');
  let made: Issue | undefined;
  const fits = $derived(title.trim().length >= 1 && title.length <= 256);

  async function send(): Promise<HubResult<unknown>> {
    const result = await api.createIssue(repository, title.trim(), body);
    if (result.ok) made = result.value;
    return result;
  }
</script>

<!-- A new issue on GitHub (PRJ-028). The hub sends it once: after "could not be reached" it may exist, so look first. -->
<ChangeDialog
  open
  title="New issue"
  body="Anima.ing makes the issue on GitHub through your GitHub App."
  confirmLabel="Create issue"
  ready={fits}
  onConfirm={send}
  onDone={() => {
    if (made !== undefined) onDone(made);
  }}
  {onCancel}
>
  <Field label="Title" constraint="1 to 256 characters">
    {#snippet children(link)}<Input {...link} maxlength={256} bind:value={title} />{/snippet}
  </Field>
  <label class="body">
    <span>Description</span>
    <textarea rows="6" bind:value={body} maxlength={65536}></textarea>
  </label>
</ChangeDialog>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  textarea {
    box-sizing: border-box;
    padding: var(--space-2) var(--space-3);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text);
    font: inherit;
    resize: vertical;
  }

  textarea:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }
</style>
