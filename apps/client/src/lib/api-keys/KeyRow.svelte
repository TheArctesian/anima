<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { ApiKey } from './api';
  import { isExpired, lastUsedText, scopeLabel, shownDate } from './listing';

  interface Props {
    key: ApiKey;
    /** The time the expiry mark is measured against. */
    expiredAt: Date;
    onRevoke: () => void;
  }

  const { key, expiredAt, onRevoke }: Props = $props();
  const expired = $derived(isExpired(key, expiredAt));
</script>

<li aria-label={key.name}>
  <div class="head">
    <strong>{key.name}</strong>
    <FormButton tabindex={0} tone="danger" aria-label="Revoke {key.name}" onclick={onRevoke}>Revoke</FormButton>
  </div>
  <dl>
    <div><dt>Scope</dt><dd>{scopeLabel(key.scope)}</dd></div>
    <div><dt>Prefix</dt><dd class="mono">{key.prefix}</dd></div>
    <div>
      <dt>Expires</dt>
      <dd>
        <time datetime={key.expiresAt}>{shownDate(key.expiresAt)}</time>
        {#if expired}<span class="expired">Expired</span>{/if}
      </dd>
    </div>
    <div>
      <dt>Last used</dt>
      <dd>
        {#if key.lastUsedAt === null}Never{:else}<time datetime={key.lastUsedAt}>{lastUsedText(key)}</time>{/if}
      </dd>
    </div>
  </dl>
</li>

<style>
  li {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
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
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  dd {
    margin: 0;
    font-size: var(--text-sm);
  }

  .mono {
    font-family: var(--font-mono);
  }

  /* An expired key is marked with the word, and a bar beside it, never by colour alone (UI-003). */
  .expired {
    margin-inline-start: var(--space-1);
    padding-inline-start: var(--space-1);
    border-inline-start: var(--border-focus) solid var(--status-warning);
    font-weight: var(--weight-bold);
  }
</style>
