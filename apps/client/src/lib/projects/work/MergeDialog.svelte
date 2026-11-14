<script lang="ts">
  import Field from '$lib/forms/field/Field.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import Select from '$lib/forms/input/Select.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import ChangeDialog from '../forge/ChangeDialog.svelte';
  import type { ForgeApi } from '../forge/api';
  import type { MergeMethod, PullRequest, PullRequestMerge } from '../forge/types';
  import { shortSha } from '../forge/words';

  interface Props {
    api: ForgeApi;
    /** One pull request, or several clean ones from this repository (PRJ-033). */
    pulls: readonly PullRequest[];
    /** The merge methods the repository allows; the first is offered. */
    methods: readonly MergeMethod[];
    /** The result for each, in order; one merged alone counts as merged. */
    onDone: (results: readonly PullRequestMerge[]) => void;
    onCancel: () => void;
  }

  const { api, pulls, methods, onDone, onCancel }: Props = $props();
  const METHOD_WORDS: Record<MergeMethod, string> = { MERGE: 'Merge commit', SQUASH: 'Squash and merge', REBASE: 'Rebase and merge' };
  // svelte-ignore state_referenced_locally
  let method = $state<MergeMethod>(methods[0] ?? 'MERGE');
  let typed = $state('');
  let results: readonly PullRequestMerge[] = [];
  const one = $derived(pulls.length === 1 ? pulls[0] : undefined);
  /** A pull request from a fork merges only after its repository is typed (PRO-013). */
  const fork = $derived(one?.fromFork === true ? (one.headRepository ?? '') : null);
  const unstable = $derived(one?.mergeability?.state === 'UNSTABLE');
  const lines = $derived(pulls.map((pull) => `#${String(pull.number)} ${pull.title} · ${pull.headBranch} at ${shortSha(pull.headSha)}`));

  async function send(): Promise<HubResult<unknown>> {
    if (one !== undefined) {
      const result = await api.merge(one, method, { allowFailingChecks: unstable, forkConfirmed: fork });
      if (result.ok) results = [{ number: one.number, outcome: 'MERGED', message: `#${String(one.number)} merged.` }];
      return result;
    }
    const result = await api.mergeMany(pulls, method);
    if (result.ok) results = result.value;
    return result;
  }
</script>

<ChangeDialog
  open
  title={one === undefined ? `Merge ${String(pulls.length)} pull requests` : `Merge #${String(one.number)}`}
  body={one === undefined
    ? 'Each is merged in turn at the commit shown; one that cannot merge is left and the rest go on.'
    : unstable
      ? 'A check that is not required failed. It merges at the commit shown; if the branch moved, GitHub refuses.'
      : 'It merges at the commit shown; if the branch moved since, GitHub refuses.'}
  {lines}
  confirmLabel={one === undefined ? `Merge ${String(pulls.length)}` : 'Merge'}
  tone={unstable || fork !== null ? 'danger' : 'primary'}
  ready={fork === null || (fork !== '' && typed === fork)}
  onConfirm={send}
  onDone={() => onDone(results)}
  {onCancel}
>
  {#if methods.length > 1}
    <Field label="Method">
      {#snippet children(link)}
        <Select {...link} bind:value={method}>
          {#each methods as each (each)}<option value={each}>{METHOD_WORDS[each]}</option>{/each}
        </Select>
      {/snippet}
    </Field>
  {/if}
  {#if fork !== null}
    <Field label="Type {fork === '' ? 'the fork' : fork} to merge" constraint="This pull request comes from another repository.">
      {#snippet children(link)}<Input {...link} mono autocomplete="off" bind:value={typed} />{/snippet}
    </Field>
  {/if}
</ChangeDialog>
