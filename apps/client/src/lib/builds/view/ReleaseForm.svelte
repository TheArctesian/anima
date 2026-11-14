<script lang="ts">
  import Field from '$lib/forms/field/Field.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Input from '$lib/forms/input/Input.svelte';
  import type { HubError } from '$lib/hub/hub';
  import type { BuildTarget } from './api';
  import { normalized, refusalText, validateRelease, withTarget, type ReleaseErrors } from './form';
  import { TARGETS } from './release';

  interface Props {
    /** The version the hub proposes, or `null` before the first release. */
    proposed: string | null;
    /** Asks the hub for the release; the answer is its refusal, or `null` when the release started (BLD-001, BLD-002). */
    onrelease: (version: string, commit: string, targets: readonly BuildTarget[]) => Promise<HubError | null>;
  }

  const { proposed, onrelease }: Props = $props();

  const ORDER = TARGETS.map((target) => target.value);
  let version = $state('');
  let edited = false;
  let commit = $state('');
  let targets = $state<readonly BuildTarget[]>(ORDER);
  let shown = $state<ReleaseErrors>({});
  let refusal = $state('');
  let busy = $state(false);

  // The proposal fills the field until the operator types in it.
  $effect(() => {
    if (proposed !== null && !edited) version = proposed;
  });

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy) return;
    shown = validateRelease({ version, commit, targets });
    refusal = '';
    if (Object.keys(shown).length > 0) return;
    busy = true;
    try {
      const sent = normalized({ version, commit, targets });
      const error = await onrelease(sent.version, sent.commit, sent.targets);
      if (error !== null) refusal = refusalText(error);
      else commit = '';
    } finally {
      busy = false;
    }
  }
</script>

<form aria-label="Release form" novalidate onsubmit={submit}>
  <Field label="Version" error={shown.version}>
    {#snippet children(link)}
      <Input
        {...link}
        mono
        bind:value={version}
        oninput={() => {
          edited = true;
        }}
      />
    {/snippet}
  </Field>
  <Field label="Commit" constraint="The full commit of main: 40 hex digits." error={shown.commit}>
    {#snippet children(link)}
      <Input {...link} mono bind:value={commit} placeholder="0123456789abcdef0123456789abcdef01234567" />
    {/snippet}
  </Field>
  <fieldset>
    <legend>Targets</legend>
    {#each TARGETS as target (target.value)}
      <label class="target">
        <input
          type="checkbox"
          checked={targets.includes(target.value)}
          onchange={(event) => {
            targets = withTarget(targets, target.value, event.currentTarget.checked, ORDER);
          }}
        />
        {target.label}
      </label>
    {/each}
    <div role="status">{#if shown.targets}<p class="error">{shown.targets}</p>{/if}</div>
  </fieldset>
  <div role="alert">{#if refusal}<p class="error">{refusal}</p>{/if}</div>
  <FormButton type="submit" tone="primary" tabindex={0} {busy}>Release {version}</FormButton>
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    padding: 0;
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .target {
    display: flex;
    gap: var(--space-2);
    align-items: center;
    min-block-size: var(--target-min-desktop);
  }

  .error {
    margin: 0;
    color: var(--status-down-text);
    font-size: var(--text-sm);
  }
</style>
