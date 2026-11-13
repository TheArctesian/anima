<script lang="ts">
  import Disclosure from '$lib/forms/disclosure/Disclosure.svelte';
  import BulkActions from './BulkActions.svelte';
  import { countsText, type McpChange, type ToolGroup } from './permissions';
  import ToolRow from './ToolRow.svelte';

  interface Props {
    group: ToolGroup;
    onrequest: (change: McpChange) => void;
  }

  const { group, onrequest }: Props = $props();
</script>

<!-- The counts are in the label, in words, so a closed group still says what it holds (UI-003). -->
<Disclosure label="{group.label}: {countsText(group.counts)}" level={3}>
  <BulkActions scope="{group.label} tools" toolset={group.toolset} {onrequest} />
  <ul>
    {#each group.tools as tool (tool.tool)}
      <ToolRow {tool} onchoose={(state) => { onrequest({ type: 'choose', names: [tool.tool], state }); }} />
    {/each}
  </ul>
</Disclosure>

<style>
  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
