<script lang="ts">
  import { describedBy, inputId } from '$lib/devices/form/form/errors';
  import { osLabel } from '$lib/devices/os/os';
  import OsIcon from '$lib/devices/os/OsIcon.svelte';
  import SaveMark from '$lib/forms/feedback/SaveMark.svelte';
  import Select from '$lib/forms/input/Select.svelte';
  import type { OperatingSystem } from '$lib/hub/generated/graphql';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import SettingsSection from '$lib/settings/SettingsSection.svelte';
  import type { SettingsDraft } from '../draft.svelte';
  import { SECTIONS } from '../sections';
  import TagsControl from './TagsControl.svelte';
  import TextRow from './TextRow.svelte';

  const { draft }: { draft: SettingsDraft } = $props();

  const SYSTEMS: readonly OperatingSystem[] = ['LINUX', 'MACOS', 'WINDOWS', 'OTHER'];
  const KINDS = ['server', 'router', 'switch', 'workstation', 'laptop', 'printer'] as const;
  /** DEV-025: the hub reads the operating system of a device at Other whose metrics come over SSH. */
  const detectsOs = $derived(draft.values.os === 'OTHER' && draft.values.metricsSource === 'SSH');
  const tagsError = $derived(draft.errorOf('tags'));
</script>

{#snippet osMark()}<SaveMark mark={draft.marks['os']} />{/snippet}
{#snippet osHint()}<p class="hint" id="{inputId('os')}-hint">Detected over SSH at the next collection.</p>{/snippet}
{#snippet tagsMark()}<SaveMark mark={draft.marks['tags']} />{/snippet}
{#snippet tagsHint()}<p class="hint" id="{inputId('tags')}-hint">Return or a comma adds a tag.</p>{/snippet}
{#snippet tagsMessage()}<p role="alert" id="{inputId('tags')}-error">{tagsError?.message}</p>{/snippet}

<SettingsSection {...SECTIONS.device}>
  <TextRow {draft} key="name" label="Name" />
  <TextRow {draft} key="host" label="Address" about="A host name or an IP address." mono />
  <TextRow {draft} key="kind" label="Kind" list="device-kind-options" />
  <datalist id="device-kind-options">
    {#each KINDS as kind (kind)}<option value={kind}></option>{/each}
  </datalist>
  <!-- A select saves when it is left or Return is pressed, as text does (D17). It cannot show a logo, so the chosen
       system's logo sits beside it; the select names it (UI-003). -->
  <SettingRow label="Operating system" for={inputId('os')} wide status={osMark} hint={detectsOs ? osHint : undefined}>
    <span class="system">
      <OsIcon os={draft.values.os} decorative />
      <Select
        id={inputId('os')}
        aria-describedby={describedBy('os', detectsOs, false)}
        bind:value={draft.values.os}
        onblur={() => void draft.commit('os')}
        onkeydown={(event) => {
          if (event.key === 'Enter') void draft.commit('os');
        }}
      >
        {#each SYSTEMS as system (system)}<option value={system}>{osLabel(system)}</option>{/each}
      </Select>
    </span>
  </SettingRow>
  <SettingRow label="Tags" for={inputId('tags')} wide status={tagsMark} hint={tagsHint} message={tagsError === undefined ? undefined : tagsMessage}>
    <TagsControl
      id={inputId('tags')}
      bind:tags={draft.values.tags}
      aria-describedby={describedBy('tags', true, tagsError !== undefined)}
      onchange={() => void draft.commit('tags')}
    />
  </SettingRow>
  <TextRow {draft} key="macAddress" label="MAC address" about="Optional. Lets Anima wake the device." placeholder="aa:bb:cc:dd:ee:ff" mono />
</SettingsSection>

<style>
  .system {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-inline-size: 0;
  }

  .system > :global(.select) {
    flex: 1;
  }
</style>
