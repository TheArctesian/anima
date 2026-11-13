<script lang="ts">
  import Disclosure from '$lib/forms/disclosure/Disclosure.svelte';
  import BulkActions from './BulkActions.svelte';
  import { groupLabel, type McpChange, type ToolGroup } from './permissions';
  import ToolRow from './ToolRow.svelte';

  interface Props {
    group: ToolGroup;
    onrequest: (change: McpChange) => void;
  }

  const { group, onrequest }: Props = $props();
</script>

<!-- The counts are in the label, in words, so a closed group still says what it holds (UI-003). -->
<Disclosure label={groupLabel(group)} level={3}>
  <BulkActions scope="{group.label} tools" toolset={group.toolset} {onrequest} />
  {#if group.tools.some((tool) => tool.needsSignIn)}
    <p class="note">
      A tool marked "Needs sign-in" works only when <code>anima mcp</code> runs with your own sign-in. With an API key the agent never gets it, whatever
      is set here.
    </p>
  {/if}
  <ul>
    {#each group.tools as tool (tool.tool)}
      <ToolRow {tool} onchoose={(state) => { onrequest({ type: 'choose', names: [tool.tool], state }); }} />
    {/each}
  </ul>
</Disclosure>

<style>
  .note {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  code {
    font-family: var(--font-mono);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
