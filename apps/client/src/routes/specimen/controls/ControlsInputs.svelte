<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import Select from '$lib/forms/input/Select.svelte';
</script>

<section aria-labelledby="specimen-inputs">
  <h3 id="specimen-inputs">Inputs</h3>
  <div class="row">
    <Input aria-label="Empty with placeholder" placeholder="for example, admin" />
    <Input aria-label="Filled" value="admin" />
    <Input aria-label="Invalid" value="not a host" invalid />
    <Input aria-label="Disabled" value="locked" disabled />
    <Input aria-label="Large" size="lg" placeholder="40 px, for sign-in" />
  </div>
  <div class="row">
    <Input aria-label="Machine text" mono value="hub.anima.ing" />
    <Input aria-label="Fingerprint" mono value="SHA256:3f9aG0n+6hQ2" />
    <Input aria-label="Number" type="number" value={2222} />
    <Select aria-label="Select"><option>Password</option><option>Private key</option></Select>
    <Select aria-label="Select, invalid" invalid><option>Choose</option></Select>
    <Select aria-label="Select, disabled" disabled><option>Locked</option></Select>
  </div>

  <h4>One row: input, select and button share a height and baseline</h4>
  <div class="row">
    <Input aria-label="Host" placeholder="Host" />
    <Select aria-label="Protocol"><option>SSH</option><option>RDP</option></Select>
    <FormButton tone="primary">Test connection</FormButton>
    <FormButton tone="ghost">Cancel</FormButton>
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

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }
</style>
