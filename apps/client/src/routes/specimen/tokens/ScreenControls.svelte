<script lang="ts">
  import StatusBadge from '$lib/devices/status/StatusBadge.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import Icon from '$lib/icons/Icon.svelte';
</script>

<!-- The controls in the website's language (UR-7): the four button tones, status, a planned mark, a search field. -->
<div class="line">
  <FormButton tone="primary" icon="carbon:add">Add device</FormButton>
  <FormButton icon="carbon:terminal">Connect SSH</FormButton>
  <FormButton tone="ghost">Cancel</FormButton>
  <FormButton tone="danger">Delete device</FormButton>
  <StatusBadge status="DOWN" deviceId="specimen-down" />
  <StatusBadge status="DEGRADED" deviceId="specimen-degraded" />
  <StatusBadge status="UP" deviceId="specimen-up" />
  <span class="planned"><Icon name="carbon:time" size="sm" />Planned</span>
</div>
<label class="search">
  <span class="visually-hidden">Search devices</span>
  <Input icon="carbon:search" placeholder="Search by name" />
</label>

<style>
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }

  .planned {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--planned);
    font-size: var(--text-xs);
  }

  .search {
    display: flex;
    inline-size: min(18rem, 100%);
    margin-block-start: var(--space-4);
  }

  .search :global(.with-icon) {
    flex: 1 1 auto;
  }

  .visually-hidden {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
