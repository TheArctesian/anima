<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { VpnNode } from '../api';
  import { keyText, pathLabel } from '../picture';

  interface Props {
    node: VpnNode;
    now: Date;
  }

  const { node, now }: Props = $props();
  const id = $props.id();
</script>

<!-- The VPN facts of a device: the card form of the other Overview cards (VPN-005); classes are from overview/geometry.css. -->
<section class="overview-card vpn" aria-labelledby="{id}-vpn">
  <h2 id="{id}-vpn" class="overview-card-title"><Icon name="nf-md-lan_connect" />VPN</h2>
  <dl class="facts">
    <div>
      <dt>Address</dt>
      <dd>{node.address}</dd>
    </div>
    <div>
      <dt>Path</dt>
      <dd>{pathLabel(node.path, node.relayRegion)}</dd>
    </div>
    <div>
      <dt>Key</dt>
      <dd>{keyText(node, now)}</dd>
    </div>
    <div>
      <dt>OS</dt>
      <dd>{node.os}</dd>
    </div>
  </dl>
</section>

<style>
  .vpn {
    margin-block-end: var(--space-4);
  }

  .facts {
    display: grid;
    gap: var(--space-2);
    margin: 0;
  }

  .facts div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    min-block-size: var(--target-min-desktop);
  }

  .facts dt {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .facts dd {
    margin: 0;
    color: var(--text);
    text-align: end;
  }
</style>
