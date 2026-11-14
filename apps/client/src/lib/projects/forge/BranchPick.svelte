<script lang="ts">
  import { untrack } from 'svelte';
  import Select from '$lib/forms/input/Select.svelte';
  import type { ForgeApi } from './api';
  import type { Branch } from './types';

  interface Props {
    api: ForgeApi;
    repository: string;
    /** The branch shown; the repository's default branch until the operator picks another. */
    value: string;
    onChange: (branch: string) => void;
  }

  const { api, repository, value, onChange }: Props = $props();
  let branches = $state.raw<readonly Branch[]>([]);

  $effect(() => {
    void untrack(() => api.branches(repository)).then((result) => {
      if (result.ok) branches = result.value.rows;
    });
  });

  /** The branch shown stays a choice even before the branches arrive or when it is not among the first hundred. */
  const names = $derived(branches.some((each) => each.name === value) ? branches.map((each) => each.name) : [value, ...branches.map((each) => each.name)]);
</script>

<Select
  aria-label="Branch"
  mono
  {value}
  onchange={(event) => {
    onChange(event.currentTarget.value);
  }}
>
  {#each names as name (name)}<option value={name}>{name}</option>{/each}
</Select>
