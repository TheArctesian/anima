<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';

  /** The icons the pages use, by what each one stands for (UR-7, UI-014). */
  const ICONS: readonly (readonly [IconName, string])[] = [
    ['carbon:bare-metal-server', 'Devices'],
    ['carbon:screen', 'Live sessions'],
    ['carbon:settings', 'Settings'],
    ['carbon:terminal', 'SSH'],
    ['carbon:screen-map', 'RDP, VNC'],
    ['carbon:power', 'Wake'],
    ['carbon:activity', 'Checks'],
    ['carbon:catalog', 'Logs'],
    ['carbon:vpn', 'Private network'],
    ['carbon:network-3', 'Hub'],
    ['carbon:fingerprint-recognition', 'Host key'],
    ['carbon:certificate', 'Certificate'],
    ['carbon:api-key', 'API keys'],
    ['carbon:warning-alt', 'Needs attention'],
    ['carbon:chip', 'Hardware'],
    ['carbon:data-share', 'Integrations'],
    ['simple-icons:apple', 'macOS'],
    ['simple-icons:linux', 'Linux'],
    ['simple-icons:windows', 'Windows'],
    ['simple-icons:ubuntu', 'Ubuntu'],
    ['simple-icons:debian', 'Debian'],
    ['simple-icons:raspberrypi', 'Raspberry Pi'],
  ];
</script>

<ul>
  {#each ICONS as [name, meaning] (meaning)}
    <li><Icon {name} size="md" />{meaning}</li>
  {/each}
</ul>

<style>
  ul {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr));
    gap: var(--space-3) var(--space-4);
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
</style>
