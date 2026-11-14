<script lang="ts" generics="T">
  import type { Snippet } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import Section from '$lib/page/Section.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import Notice from '$lib/states/Notice.svelte';
  import { readWords, type ForgeList } from './list.svelte';

  interface Props {
    title: string;
    icon: IconName;
    list: ForgeList<T>;
    /** What the section says when the list is empty, for example "No open pull requests". */
    empty: string;
    emptyLine?: string;
    now: () => Date;
    /** Controls over the rows: a filter, a "New issue" button. */
    tools?: Snippet;
    children: Snippet;
  }

  const { title, icon, list, empty, emptyLine = '', now, tools, children }: Props = $props();
</script>

{#snippet refresh()}
  <span class="read">
    {#if list.read !== null}<span class:stale={list.read.stale}>{readWords(list.read, now())}</span>{/if}
    <FormButton tone="ghost" size="sm" busy={list.refreshing} onclick={() => void list.load(true)}>
      <Icon name="carbon:renew" size="sm" />Refresh
    </FormButton>
  </span>
{/snippet}

<!-- One list read from GitHub (PRJ-031, PRJ-032): loading, failed, empty or its rows, and when it was read with Refresh. -->
<Section {title} {icon} action={refresh}>
  {#if tools}<div class="tools">{@render tools()}</div>{/if}
  {#if list.status === 'loading'}
    <LoadingState label="Loading {title.toLowerCase()}" rows={3} />
  {:else if list.status === 'failed'}
    <ErrorState message={list.error?.message ?? 'The hub did not answer.'} onRetry={() => void list.load()} />
  {:else}
    {#if list.error !== null}<Notice tone="warning" title="Refresh failed" line={list.error.message} />{/if}
    {#if list.read?.stale === true && list.read.problem}<Notice tone="warning" title="GitHub did not answer" line={list.read.problem} />{/if}
    {#if list.rows.length === 0}
      <EmptyState title={empty} line={emptyLine} />
    {:else}
      {@render children()}
      {#if list.next !== null}
        <div class="more">
          <FormButton tone="ghost" size="sm" busy={list.loadingMore} onclick={() => void list.more()}>Load more</FormButton>
        </div>
      {/if}
    {/if}
  {/if}
</Section>

<style>
  .read {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }

  .stale {
    color: var(--status-warning-text);
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
  }

  .more {
    display: flex;
    justify-content: center;
  }
</style>
