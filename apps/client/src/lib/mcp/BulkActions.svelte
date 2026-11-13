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
  }

  const { scope, toolset, onrequest }: Props = $props();
</script>

<div class="bulk" role="group" aria-label="Bulk actions, {scope}">
  {#each bulkActions(toolset) as action (action.label)}
    <FormButton size="sm" aria-label="{action.label}, {scope}" onclick={() => { onrequest(action.change); }}>{action.label}</FormButton>
  {/each}
</div>

<style>
  .bulk {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
</style>
