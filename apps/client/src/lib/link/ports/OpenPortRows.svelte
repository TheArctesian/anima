<script lang="ts">
  import { onMount } from 'svelte';
  import { linkPorts } from '$lib/desktop/link/shell';
  import Switch from '$lib/forms/switch/Switch.svelte';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { portRows, systemOf, type PortRow } from './ports';

  // The ports of this computer that its hub may reach through the link (HMD-048, HMD-049): off until the person turns
  // one on, one click each. The hub cannot change them. Shown only when this computer has joined a hub.
  const system = systemOf(navigator.userAgent);
  let rows = $state<readonly PortRow[] | null>(null);
  let busy = $state(false);
  let problem = $state<string | null>(null);
  /** Goes up when a change failed: the switches are drawn again from the list, since each keeps its own state. */
  let drawn = $state(0);

  onMount(() => {
    void linkPorts().then((result) => {
      if (result.ok) rows = portRows(system, result.value.open);
    });
  });

  async function turn(port: number, on: boolean): Promise<void> {
    busy = true;
    const result = await linkPorts({ port, on });
    busy = false;
    if (result.ok) {
      rows = portRows(system, result.value.open);
      problem = null;
    } else {
      // The switch shows the list as it was; the person sees why.
      drawn += 1;
      problem = `Could not turn the port ${on ? 'on' : 'off'}. ${result.message}`;
    }
  }
</script>

{#if rows !== null}
  {#each rows as row (`${String(row.port)}-${String(drawn)}`)}
    <SettingRow label={row.label} for="open-port-{row.port}">
      {#snippet hint()}<p class="hint" id="open-port-{row.port}-hint">{row.needs}</p>{/snippet}
      <Switch
        id="open-port-{row.port}"
        checked={row.on}
        describedby="open-port-{row.port}-hint"
        disabled={busy}
        onchange={(on) => {
          void turn(row.port, on);
        }}
      />
    </SettingRow>
  {/each}
  <p class="problem" role="alert">{problem ?? ''}</p>
{/if}

<style>
  .hint {
    margin: 0;
  }

  .problem {
    margin: 0;
    color: var(--danger);
    font-size: var(--text-sm);
  }

  .problem:empty {
    display: none;
  }
</style>
