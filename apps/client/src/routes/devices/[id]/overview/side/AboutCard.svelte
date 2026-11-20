<script lang="ts">
  import '../geometry.css';
  import { aboutFacts, type SideDevice } from './side';

  const { device }: { device: SideDevice } = $props();
  const facts = $derived(aboutFacts(device));
</script>

<!-- What the device is: its system, its kind, its tags, its daemon and its session limit (R-224). -->
<section class="overview-card about" aria-labelledby="overview-about">
  <h2 id="overview-about" class="overview-card-title">About</h2>
  <dl>
    {#each facts as fact (fact.label)}
      <div><dt>{fact.label}</dt><dd>{fact.text}</dd></div>
    {/each}
    <div>
      <dt>Tags</dt>
      <dd class="tags">
        {#each device.tags as tag (tag)}<span class="tag">{tag}</span>{:else}None{/each}
      </dd>
    </div>
  </dl>
</section>

<style>
  dl {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--text-sm);
  }

  dl > div {
    display: grid;
    grid-template-columns: calc(var(--space-16) + var(--space-4)) minmax(0, 1fr);
    gap: var(--space-3);
  }

  dt {
    color: var(--text-secondary);
  }

  dd {
    margin: 0;
    color: var(--text);
    overflow-wrap: anywhere;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .tag {
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--surface-sunken);
    color: var(--text-secondary);
  }
</style>
