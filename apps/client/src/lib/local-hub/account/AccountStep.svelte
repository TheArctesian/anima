<script lang="ts">
  import { onMount, tick } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Field from '$lib/forms/field/Field.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import { checkAccount, firstProblem, PASSWORD_MIN, type AccountProblems } from './account';

  interface Props {
    /** The step's heading; focus moves to it when the step shows. */
    heading: string;
    /** The name and password the operator chose. The page keeps the password only until it has signed in (LOG-007). */
    onnext: (account: { name: string; password: string }) => void;
  }

  const { heading, onnext }: Props = $props();
  let name = $state('');
  let password = $state('');
  let repeat = $state('');
  let problems = $state<AccountProblems>({});
  let title: HTMLElement | undefined = $state();
  let form: HTMLFormElement | undefined = $state();

  onMount(() => {
    title?.focus();
  });

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    problems = checkAccount({ name, password, repeat });
    const first = firstProblem(problems);
    if (first === undefined) {
      const chosen = { name, password };
      // The secrets leave this step's state as soon as they are handed on.
      password = '';
      repeat = '';
      onnext(chosen);
      return;
    }
    await tick();
    form?.querySelectorAll<HTMLInputElement>('input')[['name', 'password', 'repeat'].indexOf(first)]?.focus();
  }
</script>

<div class="step">
  <h2 tabindex="-1" bind:this={title}>{heading}</h2>
  <p>You sign in with this name, password and an authenticator code.</p>
  <form bind:this={form} onsubmit={submit} novalidate>
    <Field label="User name" constraint="Letters, digits, dots, underscores or hyphens." error={problems.name}>
      {#snippet children(link)}<Input {...link} autocomplete="username" spellcheck="false" autocapitalize="off" bind:value={name} />{/snippet}
    </Field>
    <Field label="Password" constraint="At least {PASSWORD_MIN} characters." error={problems.password}>
      {#snippet children(link)}<Input {...link} type="password" autocomplete="new-password" bind:value={password} />{/snippet}
    </Field>
    <Field label="Repeat password" error={problems.repeat}>
      {#snippet children(link)}<Input {...link} type="password" autocomplete="new-password" bind:value={repeat} />{/snippet}
    </Field>
    <FormButton type="submit" tone="primary" size="lg">Continue</FormButton>
  </form>
</div>

<style>
  .step,
  form {
    display: grid;
    gap: var(--space-4);
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-medium);
    line-height: var(--leading-tight);
  }

  h2:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  p {
    margin: 0;
    color: var(--text-secondary);
    line-height: var(--leading-normal);
  }
</style>
