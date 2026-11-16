<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { LinkDevice } from './api';
  import { expiryText, pathText } from './devices';

  const { device, onrevoke }: { device: LinkDevice; onrevoke: () => void } = $props();
</script>

<li aria-label={device.name}>
  <div class="head">
    <strong>{device.name}</strong>
    <FormButton tabindex={0} size="sm" tone="danger" aria-label="Revoke {device.name}" onclick={onrevoke}>Revoke</FormButton>
  </div>
  <dl>
    <div><dt>Device key</dt><dd class="mono">{device.fingerprint}</dd></div>
    <div><dt>Path</dt><dd>{pathText(device)}</dd></div>
    <div><dt>Key expires</dt><dd>{#if device.expiresAt === null}No expiry{:else}<time datetime={device.expiresAt}>{expiryText(device.expiresAt)}</time>{/if}</dd></div>
  </dl>
</li>

<style>
  li {
    display: grid;
    gap: var(--space-2);
    padding: var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  strong {
    min-inline-size: 0;
    overflow-wrap: anywhere;
  }

  dl {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-6);
    margin: 0;
  }

  dt {
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }

  dd {
    margin: 0;
    font-size: var(--text-sm);
  }

  .mono {
    font-family: var(--font-mono);
  }
</style>
