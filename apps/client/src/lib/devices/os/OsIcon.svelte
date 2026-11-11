<script lang="ts">
  import type { OperatingSystem } from '$lib/hub/generated/graphql';
  import Icon from '$lib/icons/Icon.svelte';
  import { osIcon, osLabel } from './os';

  interface Props {
    os: OperatingSystem;
    size?: 'sm' | 'md' | 'lg';
    /** True where the system's name is already written beside the logo, so a screen reader does not hear it twice. */
    decorative?: boolean;
    /** The os-release ID the hub read over SSH; a Linux device shows its distribution's logo when it has one. */
    distribution?: string | null;
  }

  const { os, size = 'md', decorative = false, distribution = null }: Props = $props();
  const icon = $derived(osIcon(os, distribution));
  const label = $derived(osLabel(os, distribution));
</script>

<!-- The logo has the system's name as its accessible name and tooltip unless the name is written beside it (UI-015). -->
{#if decorative}
  <span class="os"><Icon name={icon} {size} /></span>
{:else}
  <span class="os" role="img" aria-label={label} title={label}><Icon name={icon} {size} /></span>
{/if}

<style>
  .os {
    flex: none;
    display: inline-flex;
    align-items: center;
    color: var(--text-muted);
  }
</style>
