<script lang="ts">
  /** The colour roles the website's language leans on (UR-7), each with what it is for. */
  const SWATCHES = [
    ['surface-sunken', 'Ground'],
    ['surface-base', 'Inputs'],
    ['surface-raised', 'Menus, dialogs'],
    ['border-subtle', 'Hairline rows'],
    ['border-strong', 'Control edges'],
    ['text', 'Text'],
    ['text-secondary', 'Secondary'],
    ['text-tertiary', 'Tertiary'],
    ['text-quiet', 'Quiet · new'],
    ['accent', 'Accent'],
    ['chart-series-3', 'Series 1'],
    ['chart-series-2', 'Series 2'],
    ['chart-series-1', 'Series 3'],
    ['chart-series-4', 'Series 4'],
    ['status-up', 'Up'],
    ['status-down', 'Down'],
    ['status-degraded', 'Degraded'],
    ['status-warning', 'Warning'],
    ['planned', 'Planned · new'],
  ] as const;
</script>

<ul>
  {#each SWATCHES as [role, use] (role)}
    <li>
      <span class="chip" style:background="var(--{role})" aria-hidden="true"></span>
      {use}
      <code>{role}</code>
    </li>
  {/each}
</ul>

<style>
  ul {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr));
    gap: var(--space-2) var(--space-10);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding-block: var(--space-2);
    border-block-end: var(--border-thin) solid var(--border-subtle);
    font-size: var(--text-sm);
  }

  .chip {
    flex: none;
    inline-size: var(--space-8);
    block-size: var(--space-8);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-sm);
  }

  code {
    margin-inline-start: auto;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
  }
</style>
