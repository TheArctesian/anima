<script lang="ts">
  import { listRow } from '$lib/motion/roles/transitions';
  import type { VpnNode } from '../api';
  import { keyText, keyTone, sortedNodes } from '../picture';

  interface Props {
    /** The hub's own node, listed first. */
    hub: VpnNode;
    nodes: readonly VpnNode[];
    now: Date;
  }

  const { hub, nodes, now }: Props = $props();
  const rows = $derived([hub, ...sortedNodes(nodes)]);
</script>

<!-- Each node and when its key expires, in words beside a tone dot (VPN-010, VPN-011, UI-003). -->
<ul class="nodes" aria-label="VPN nodes">
  {#each rows as node, index (node.address)}
    <li class="row" transition:listRow={{ index }} aria-label={node.name}>
      <span class="name">{node.name}</span>
      <span class="role">{node.role.toLowerCase()}</span>
      <span class="address">{node.address}</span>
      <span class="key {keyTone(node.keyState)}"><span class="dot" aria-hidden="true"></span>{keyText(node, now)}</span>
      {#if node !== hub && !node.inAnima}
        <span class="strange">Not in Anima</span>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .nodes {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: var(--space-1) var(--space-4);
    align-items: center;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border);
  }

  .name {
    font-size: var(--text-md);
    line-height: var(--leading-tight);
  }

  .role,
  .address {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .key {
    display: inline-flex;
    grid-column: 1 / -1;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-sm);
  }

  .dot {
    flex: none;
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--tone);
  }

  .up {
    --tone: var(--status-up);
  }

  .warning {
    --tone: var(--status-warning);
  }

  .down {
    --tone: var(--status-down);
  }

  .strange {
    grid-column: 1 / -1;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
</style>
