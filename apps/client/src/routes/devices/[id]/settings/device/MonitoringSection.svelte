<script lang="ts">
  import type { DeviceDetail } from '$lib/devices/api';
  import SaveMark from '$lib/forms/feedback/SaveMark.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import AgentPanel from '$lib/metrics/agent/AgentPanel.svelte';
  import type { AgentCalls } from '$lib/metrics/agent/install.svelte';
  import type { ThisComputerShell } from '$lib/desktop/agent/shell';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import SettingsSection from '$lib/settings/SettingsSection.svelte';
  import type { SettingsDraft } from '../draft.svelte';
  import { SECTIONS } from '../sections';

  interface Props {
    draft: SettingsDraft;
    device: DeviceDetail;
    /** The Install agent panel's hub calls and the hub origin the agent must reach (MET-019). */
    agent: { calls: AgentCalls; origin: string; reveal: boolean; shell: ThisComputerShell | null };
    now: () => Date;
    /** Opens the Checks tab, where the device's checks are changed. */
    onOpenChecks: () => void;
  }

  const { draft, device, agent, now, onOpenChecks }: Props = $props();

  /** D-7: only enrolment sets AGENT, so it is offered only as the saved state; picking it back sends nothing. */
  const agentSet = $derived(device.metricsSource === 'AGENT');
  const sources = $derived<readonly { value: MetricsSource; label: string }[]>([
    { value: 'NONE', label: 'None' },
    { value: 'SSH', label: 'SSH' },
    ...(agentSet ? [{ value: 'AGENT' as const, label: 'Agent' }] : []),
  ]);
</script>

{#snippet sourceMark()}<SaveMark mark={draft.marks['metricsSource']} />{/snippet}
{#snippet sourceHint()}
  <p class="hint">
    {agentSet ? 'Set by the agent. Revoke it, or pick None or SSH, to change.' : 'Where CPU, memory, disk and network figures come from.'}
  </p>
{/snippet}
{#snippet checksHint()}
  <p class="hint">New devices get one ping check: every 60 seconds, 3 retries 20 seconds apart, Degraded above 1000 ms.</p>
{/snippet}
{#snippet sourceMessage()}<p role="alert">{draft.errorOf('metricsSource')?.message}</p>{/snippet}

<SettingsSection {...SECTIONS.monitoring}>
  <SettingRow
    label="Metrics source"
    status={sourceMark}
    hint={sourceHint}
    message={draft.errorOf('metricsSource') === undefined ? undefined : sourceMessage}
  >
    <Segmented label="Metrics source" options={sources} bind:value={draft.values.metricsSource} onchange={() => void draft.commit('metricsSource')} />
  </SettingRow>
  <SettingRow label="Checks" hint={checksHint}>
    <FormButton tone="ghost" icon="carbon:arrow-right" onclick={onOpenChecks}>Open Checks</FormButton>
  </SettingRow>
</SettingsSection>
<AgentPanel
  deviceId={device.id}
  deviceName={device.name}
  host={device.host}
  source={device.metricsSource}
  calls={agent.calls}
  origin={agent.origin}
  reveal={agent.reveal}
  shell={agent.shell}
  {now}
/>
