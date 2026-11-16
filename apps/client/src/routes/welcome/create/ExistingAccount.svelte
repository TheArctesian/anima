<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Field from '$lib/forms/field/Field.svelte';
  import Input from '$lib/forms/input/Input.svelte';

  interface Props {
    /** The name and password of the account the hub already has; the code comes next (FRN-002). */
    onnext: (account: { name: string; password: string }) => void;
  }

  const { onnext }: Props = $props();
  let name = $state('');
  let password = $state('');
  let problem = $state<string | undefined>();

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    if (name.trim() === '' || password === '') {
      problem = 'Enter the user name and password.';
      return;
    }
    problem = undefined;
    const chosen = { name: name.trim(), password };
    // The password leaves this form's state as soon as it is handed on (LOG-007).
    password = '';
    onnext(chosen);
  }
</script>

<p>This computer's hub already has an account. Sign in with it.</p>
<form onsubmit={submit} novalidate>
  <Field label="User name">
    {#snippet children(link)}<Input {...link} autocomplete="username" spellcheck="false" autocapitalize="off" bind:value={name} />{/snippet}
  </Field>
  <Field label="Password" error={problem}>
    {#snippet children(link)}<Input {...link} type="password" autocomplete="current-password" bind:value={password} />{/snippet}
  </Field>
  <FormButton type="submit" tone="primary" size="lg">Continue</FormButton>
</form>

<style>
  form {
    display: grid;
    gap: var(--space-4);
  }

  p {
    margin: 0;
    line-height: var(--leading-normal);
  }
</style>
