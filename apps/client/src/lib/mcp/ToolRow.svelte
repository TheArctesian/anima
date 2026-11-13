<script lang="ts">
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import HelpTip from '$lib/overlay/helptip/HelpTip.svelte';
  import { STATES, toolLabel, type McpState, type McpTool } from './permissions';

  interface Props {
    tool: McpTool;
    onchoose: (state: McpState) => void;
  }

  const { tool, onchoose }: Props = $props();
  const id = $props.id();
  let open = $state(false);
</script>

<li>
  <div class="head">
    <div class="name">
      <strong>{toolLabel(tool.tool)}</strong>
      <code>{tool.tool}</code>
      <span class="markers">
        <span class="marker">{tool.kind === 'read' ? 'Read' : 'Change'}</span>
        {#if tool.needsSignIn}
          <span class="marker">Needs sign-in</span>
          <HelpTip label="needs sign-in">This tool works only when <code>anima mcp</code> runs with your own sign-in. An API key never lists it.</HelpTip>
        {/if}
      </span>
    </div>
    <Segmented label="{toolLabel(tool.tool)} permission" options={STATES} value={tool.state} onchange={onchoose} size="sm" />
  </div>
  <button type="button" class="more" aria-expanded={open} aria-controls="{id}-about" onclick={() => { open = !open; }}>
    {open ? 'Hide description' : 'Show description'}
  </button>
  <p id="{id}-about" class="about" hidden={!open}>{tool.description}</p>
</li>

<style>
  li {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2) var(--space-4);
  }

  .name {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  strong {
    overflow-wrap: anywhere;
  }

  code {
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    overflow-wrap: anywhere;
  }

  .markers {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  /* The kind and the sign-in need are words in an outline, never colour alone (UI-003). */
  .marker {
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }

  .more {
    align-self: flex-start;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: var(--text-sm);
    text-decoration: underline;
  }

  .more:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  .about {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .about[hidden] {
    display: none;
  }
</style>
