<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import { bulkActions, type McpChange } from './permissions';

  interface Props {
    /** What the buttons act on, for the group's name: "All tools" or "Devices". */
    scope: string;
    /** The tool set the actions are limited to; none means every tool. */
    toolset?: string;
    /** A request, not yet a change: "Allow all" is confirmed first (MCP-020). */
    onrequest: (change: McpChange) => void;
    /** The buttons are shown but do nothing, while the list they act on loads. */
    blocked?: boolean;
  }

  const { scope, toolset, onrequest, blocked = false }: Props = $props();
</script>

<div class="bulk" role="group" aria-label="Bulk actions, {scope}">
  {#each bulkActions(toolset) as action (action.label)}
    <FormButton size="sm" aria-label="{action.label}, {scope}" aria-disabled={blocked} onclick={() => { onrequest(action.change); }}>{action.label}</FormButton>
  {/each}
</div>

<style>
  .bulk {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
</style>
