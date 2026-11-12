<script lang="ts">
  import Field from '$lib/forms/field/Field.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import Select from '$lib/forms/input/Select.svelte';
</script>

<section aria-labelledby="specimen-fields">
  <h3 id="specimen-fields">Field</h3>
  <div class="columns">
    <div class="column">
      <h4>Stacked</h4>
      <Field label="Name">
        {#snippet children(link)}<Input {...link} placeholder="for example, nas" />{/snippet}
      </Field>
      <Field label="Host" constraint="A host name or an IP address.">
        {#snippet help()}Anima connects to this address. Default port 22.{/snippet}
        {#snippet children(link)}<Input {...link} mono value="nas.lan" />{/snippet}
      </Field>
      <Field label="Port" error="Use a number from 1 to 65535.">
        {#snippet children(link)}<Input {...link} type="number" value={70000} />{/snippet}
      </Field>
    </div>
    <div class="column">
      <h4>Inline</h4>
      <Field label="Sign in with" layout="inline">
        {#snippet children(link)}<Select {...link}><option>Password</option><option>Private key</option></Select>{/snippet}
      </Field>
      <Field label="Timeout" layout="inline" constraint="Seconds, 1 to 300.">
        {#snippet help()}The default is 30 seconds.{/snippet}
        {#snippet children(link)}<Input {...link} type="number" value={30} />{/snippet}
      </Field>
      <Field label="Fingerprint" layout="inline" error="This does not match the saved key.">
        {#snippet children(link)}<Input {...link} mono value="SHA256:3f9a" />{/snippet}
      </Field>
    </div>
  </div>
</section>

<style>
  section {
    display: grid;
    gap: var(--space-3);
  }

  h3,
  h4 {
    margin: 0;
  }

  .columns {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(22rem, 1fr));
    gap: var(--space-6);
  }

  .column {
    display: grid;
    align-content: start;
    gap: var(--space-4);
  }
</style>
