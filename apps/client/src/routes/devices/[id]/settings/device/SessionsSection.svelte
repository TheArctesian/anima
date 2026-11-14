<script lang="ts">
  import { contextPolicyCalls } from '$lib/devices/form/policy/calls';
  import { PolicyDraft } from '$lib/devices/form/policy/draft.svelte';
  import SaveMark from '$lib/forms/feedback/SaveMark.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Stepper from '$lib/forms/stepper/Stepper.svelte';
  import Switch from '$lib/forms/switch/Switch.svelte';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import SettingsSection from '$lib/settings/SettingsSection.svelte';
  import type { SettingsDraft } from '../draft.svelte';
  import { SECTIONS } from '../sections';

  interface Props {
    draft: SettingsDraft;
    deviceId: string;
  }

  const { draft, deviceId }: Props = $props();
  /** RA-036, RA-040: the device's session limit is read and saved apart from the device (F-3). */
  // The device is fixed for the life of the tab.
  // svelte-ignore state_referenced_locally
  const policy = new PolicyDraft(deviceId, contextPolicyCalls());
  const LIMIT = 'sessionLimit';
  /** The flag of each pair changed last, whose mark the pair's row shows. */
  const last = $state<Partial<Record<string, string>>>({});

  const PAIRS = [
    { name: 'Files', about: 'Moving files to and from the device in a session.', flags: [['upload', 'Upload'], ['download', 'Download']] },
    { name: 'Clipboard', about: 'Copying text between this computer and the device.', flags: [['copy', 'Copy from it'], ['paste', 'Paste to it']] },
  ] as const;

  /** A step saves at once, typed text when it is left or Return is pressed; a limit the hub would refuse is not sent. */
  function commitLimit(): void {
    if (!policy.check()) {
      draft.refuse(LIMIT);
      return;
    }
    if (!policy.changed) {
      draft.unmark(LIMIT);
      return;
    }
    const values = { ...policy.values };
    void draft.run(LIMIT, () => policy.save(values));
  }
</script>

{#snippet limitMark()}<SaveMark mark={draft.marks[LIMIT]} />{/snippet}
{#snippet limitHint()}<p class="hint" id="device-policy-sessionLimit-hint">The most live sessions at once, 1 to 100. File tunnels do not count.</p>{/snippet}
{#snippet limitMessage()}
  <p role="alert" id="device-policy-message">
    {policy.message}
    {#if policy.phase === 'failed' || policy.retrying}
      <FormButton size="sm" tone="ghost" aria-disabled={policy.phase !== 'failed'} aria-busy={policy.phase === 'loading'} onclick={() => void policy.load()}>Try again</FormButton>
    {/if}
  </p>
{/snippet}

<SettingsSection {...SECTIONS.sessions}>
  {#if policy.phase !== 'absent'}
    <SettingRow label="Session limit" for="device-policy-sessionLimit" status={limitMark} hint={limitHint} message={policy.message === '' ? undefined : limitMessage}>
      <Stepper
        id="device-policy-sessionLimit"
        label="Session limit"
        min={1}
        max={100}
        placeholder="None"
        disabled={policy.phase !== 'ready'}
        invalid={policy.message !== '' && policy.phase === 'ready'}
        aria-describedby="device-policy-sessionLimit-hint device-policy-message"
        bind:value={policy.values.sessionLimit}
        oncommit={commitLimit}
      />
    </SettingRow>
  {/if}
  {#each PAIRS as pair (pair.name)}
    {#snippet pairMark()}<SaveMark mark={draft.marks[`transfer.${last[pair.name] ?? ''}`]} />{/snippet}
    {#snippet pairHint()}<p class="hint">{pair.about}</p>{/snippet}
    <SettingRow label={pair.name} status={pairMark} hint={pairHint}>
      <span class="pair">
        {#each pair.flags as [flag, words] (flag)}
          <span class="flag">
            <Switch labelledby="device-transfer-{flag}" bind:checked={draft.values.transfer[flag]} onchange={() => {
                last[pair.name] = flag;
                void draft.commit(`transfer.${flag}`);
              }}
            />
            <span id="device-transfer-{flag}">{words}</span>
          </span>
        {/each}
      </span>
    </SettingRow>
  {/each}
</SettingsSection>

<style>
  .pair {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-5);
  }

  .flag {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    white-space: nowrap;
  }
</style>
