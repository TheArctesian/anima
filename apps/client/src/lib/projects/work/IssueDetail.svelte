<script lang="ts">
  import { untrack } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { dataAge, type HubError } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import ErrorState from '$lib/states/ErrorState.svelte';
  import LoadingState from '$lib/states/LoadingState.svelte';
  import type { ForgeApi } from '../forge/api';
  import { ForgeList } from '../forge/list.svelte';
  import type { Issue, IssueComment } from '../forge/types';

  interface Props {
    api: ForgeApi;
    issue: Issue;
    now: () => Date;
    /** The issue after it was closed or opened again. */
    onChange: (issue: Issue) => void;
  }

  const { api, issue, now, onChange }: Props = $props();
  const comments = new ForgeList<IssueComment>((ask) => api.issueComments(issue.repository, issue.number, ask), (row) => row.id);
  let reply = $state('');
  let busy = $state(false);
  let failure = $state.raw<HubError | null>(null);

  $effect(() => {
    untrack(() => void comments.load());
  });

  async function send(): Promise<void> {
    if (busy || reply.trim() === '') return;
    busy = true;
    failure = null;
    const result = await api.addComment(issue.repository, issue.number, reply);
    busy = false;
    if (!result.ok) {
      failure = result.error;
      return;
    }
    comments.rows = [...comments.rows, result.value];
    reply = '';
    onChange({ ...issue, comments: issue.comments + 1 });
  }

  /** Closing says the work is done; reopening needs no reason (PRJ-028). */
  async function setOpen(open: boolean): Promise<void> {
    busy = true;
    failure = null;
    const result = await api.setIssueOpen(issue.repository, issue.number, open, open ? null : 'COMPLETED');
    busy = false;
    if (result.ok) onChange(result.value);
    else failure = result.error;
  }
</script>

<!-- One issue opened in its row: its text and comments as plain text, since other people write them (PRJ-031), then a
     reply and Close or Reopen. -->
<div class="issue">
  {#if issue.body}<pre class="text">{issue.body}{#if issue.bodyCut}…{/if}</pre>{:else}<p class="none">No description.</p>{/if}
  {#if comments.status === 'loading'}
    <LoadingState label="Loading comments" rows={2} />
  {:else if comments.status === 'failed'}
    <ErrorState message={comments.error?.message ?? ''} onRetry={() => void comments.load()} />
  {:else}
    <ol class="comments" aria-label="Comments">
      {#each comments.rows as comment (comment.id)}
        <li>
          <span class="who">{comment.authorLogin} · {dataAge(new Date(comment.createdAt), now())}</span>
          <pre class="text">{comment.body}{#if comment.bodyCut}…{/if}</pre>
        </li>
      {/each}
    </ol>
  {/if}
  <label class="reply">
    <span>Comment</span>
    <textarea rows="3" bind:value={reply} placeholder="Plain text or Markdown; GitHub shows it."></textarea>
  </label>
  {#if failure !== null}<p class="failure" role="alert">{failure.message}</p>{/if}
  <div class="actions">
    <FormButton size="sm" tone="primary" busy={busy} aria-disabled={reply.trim() === ''} onclick={send}>Comment</FormButton>
    <FormButton size="sm" tone="ghost" onclick={() => void setOpen(!issue.open)}>
      <Icon name={issue.open ? 'carbon:checkmark-outline' : 'carbon:renew'} size="sm" />{issue.open ? 'Close as done' : 'Reopen'}
    </FormButton>
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- an address on GitHub -->
    <a class="out" href={issue.url} target="_blank" rel="noreferrer"><Icon name="simple-icons:github" size="sm" />Open in GitHub</a>
  </div>
</div>

<style>
  .issue {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3) 0 var(--space-2);
  }

  .text {
    margin: 0;
    color: var(--text-secondary);
    font-family: inherit;
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .none,
  .who {
    margin: 0;
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }

  .comments {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin: 0;
    padding: 0 0 0 var(--space-3);
    border-inline-start: var(--border-focus) solid var(--border-subtle);
    list-style: none;
  }

  .reply {
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

  .failure {
    margin: 0;
    color: var(--status-down-text);
    font-size: var(--text-sm);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .out {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin-inline-start: auto;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    text-decoration: none;
  }

  .out:hover {
    color: var(--text);
  }
</style>
