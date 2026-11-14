<script lang="ts">
  import { tick, untrack } from 'svelte';
  import type { DeviceDetail } from '$lib/devices/api';
  import type { ConnectionTestResult } from '$lib/devices/form/connection-test';
  import type { DeviceFormValues } from '$lib/devices/form/values';
  import type { ConnectionTestInput } from '$lib/hub/generated/graphql';
  import type { HubResult } from '$lib/hub/hub';
  import SettingsSection from '$lib/settings/SettingsSection.svelte';
  import type { SettingsSection as Place } from '../../tabs/tabs';
  import type { SettingsCalls } from '../calls';
  import type { SettingsDraft } from '../draft.svelte';
  import { SECTIONS } from '../sections';
  import RdpConnection from './RdpConnection.svelte';
  import SshConnection from './SshConnection.svelte';
  import VncConnection from './VncConnection.svelte';

  interface Props {
    draft: SettingsDraft;
    device: DeviceDetail;
    saved: DeviceFormValues;
    calls: SettingsCalls;
    /** The place a header button opened (DEV-022): a protocol's row, or `test`, the SSH connection test (D-8). */
    section: Place | null;
    onTest: (input: ConnectionTestInput) => Promise<HubResult<ConnectionTestResult>>;
  }

  const { draft, device, saved, calls, section, onTest }: Props = $props();
  let ssh = $state(untrack(() => section !== 'rdp'));
  let rdp = $state(untrack(() => section === 'rdp'));
  let vnc = $state(false);
  let root = $state<HTMLElement>();

  /** The control a place names: a protocol row's toggle, or Test connection. */
  const TARGETS = {
    ssh: '[data-connection="ssh"] [aria-expanded]',
    rdp: '[data-connection="rdp"] [aria-expanded]',
    test: '[data-connection="ssh"] [data-connection-test]',
  } as const;

  /** Each new place opens its row, then focuses and scrolls to its control (DEV-022, D-8). */
  $effect(() => {
    const wanted = section;
    if (wanted === null || wanted === 'general') return;
    untrack(() => {
      if (wanted === 'rdp') rdp = true;
      else ssh = true;
    });
    const reach = () => {
      const target = root?.querySelector<HTMLElement>(TARGETS[wanted]) ?? null;
      if (target === null) return;
      target.focus();
      if (typeof target.scrollIntoView === 'function') target.scrollIntoView({ block: 'nearest' });
    };
    // At once for a control already shown; again once a row that was closed has opened.
    reach();
    void tick().then(reach);
  });
</script>

<SettingsSection {...SECTIONS.connections}>
  <div class="connections" bind:this={root}>
    <SshConnection {draft} {device} {saved} {calls} {onTest} bind:open={ssh} />
    <VncConnection {draft} {device} {saved} {calls} {onTest} bind:open={vnc} />
    <RdpConnection {draft} {device} {saved} {calls} {onTest} bind:open={rdp} />
  </div>
</SettingsSection>

<style>
  /* Each row draws its hairline under itself; the list opens with one above the first, and the section's closing line
     is the last row's. */
  .connections {
    margin-block-end: calc(var(--border-thin) * -1);
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }
</style>
