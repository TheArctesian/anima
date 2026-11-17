<script lang="ts">
  import { untrack } from 'svelte';
  import Field from '$lib/forms/field/Field.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import Select from '$lib/forms/input/Select.svelte';
  import type { HubError } from '$lib/hub/hub';
  import ChangeDialog from '../forge/ChangeDialog.svelte';
  import type { ForgeApi } from '../forge/api';
  import type { Workflow } from '../forge/types';

  interface Props {
    api: ForgeApi;
    repository: string;
    defaultBranch: string;
    onDone: () => void;
    onCancel: () => void;
  }

  const { api, repository, defaultBranch, onDone, onCancel }: Props = $props();
  let workflows = $state.raw<readonly Workflow[] | null>(null);
  let failure = $state.raw<HubError | null>(null);
  let workflowId = $state('');
  // svelte-ignore state_referenced_locally
  let branch = $state(defaultBranch);
  const active = $derived((workflows ?? []).filter((each) => each.active));

  $effect(() => {
    void untrack(() => api.workflows(repository)).then((result) => {
      if (!result.ok) {
        failure = result.error;
        return;
      }
      workflows = result.value;
      workflowId = result.value.find((each) => each.active)?.id ?? '';
    });
  });
</script>

<!-- Starts a workflow by hand on a branch (PRJ-035). GitHub runs it with the repository's secrets, so it asks first. -->
<ChangeDialog
  open
  title="Run workflow"
  body="GitHub runs it on the branch with the repository’s secrets."
  confirmLabel="Run workflow"
  ready={workflowId !== '' && branch.trim() !== ''}
  onConfirm={() => api.dispatch(repository, workflowId, branch.trim())}
  {onDone}
  {onCancel}
>
  {#if failure !== null}
    <p class="failure" role="alert">{failure.message}</p>
  {:else if workflows === null}
    <p class="quiet">Loading workflows…</p>
  {:else if active.length === 0}
    <p class="quiet">No runnable workflows.</p>
  {:else}
    <Field label="Workflow">
      {#snippet children(link)}
        <Select {...link} bind:value={workflowId}>
          {#each active as workflow (workflow.id)}<option value={workflow.id}>{workflow.name} ({workflow.path})</option>{/each}
        </Select>
      {/snippet}
    </Field>
    <Field label="Branch">
      {#snippet children(link)}<Input {...link} mono autocomplete="off" bind:value={branch} />{/snippet}
    </Field>
  {/if}
</ChangeDialog>

<style>
  p {
    margin: 0;
    font-size: var(--text-sm);
  }

  .quiet {
    color: var(--text-tertiary);
  }

  .failure {
    color: var(--status-down-text);
  }
</style>
