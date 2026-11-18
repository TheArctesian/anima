<script lang="ts">
  import type { UsageAnswer } from './api';
  import { deviceNotes, notes } from './report/words';

  const { answer }: { answer: UsageAnswer } = $props();
  const lines = $derived(notes(answer));
  const devices = $derived(deviceNotes(answer.devices));
</script>

<!-- Everything here is text; a device's name and the hub's day are shown as text and never as markup. -->
<div class="notes">
  <ul>
    {#each lines as line (line)}<li>{line}</li>{/each}
  </ul>
  {#if devices.length > 0}
    <ul aria-label="Devices that send no counts">
      {#each devices as device (device.id)}<li><span class="device">{device.name}</span>: {device.sentence}</li>{/each}
    </ul>
  {/if}
</div>

<style>
  .notes {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .device {
    color: var(--text);
    font-weight: var(--weight-medium);
    overflow-wrap: anywhere;
  }
</style>
