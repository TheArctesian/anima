<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import { formatRunnerBytes, readRunnerReport, type RunnerReport } from '$lib/desktop/runner/report';
  import SettingRow from '$lib/settings/SettingRow.svelte';

  let report = $state<RunnerReport>();
  let loading = $state(false);
  let error = $state(false);

  async function refresh(): Promise<void> {
    loading = true;
    error = false;
    try {
      report = await readRunnerReport();
    } catch {
      report = undefined;
      error = true;
    } finally {
      loading = false;
    }
  }
</script>

<SettingRow label="Capability report">
  <FormButton onclick={refresh} busy={loading}>
    {loading ? 'Reading…' : report ? 'Refresh' : 'Read capabilities'}
  </FormButton>
  {#snippet message()}
    <p role="alert">{error ? 'Could not read this device’s capabilities. Try again.' : ''}</p>
  {/snippet}
</SettingRow>

{#if report}
  <SettingRow label="Power">{report.charging}</SettingRow>
  <SettingRow label="Thermal state">{report.thermal}</SettingRow>
  <SettingRow label="Available app memory">{formatRunnerBytes(report.availableProcessBytes)}</SettingRow>
  <SettingRow label="Available storage">{formatRunnerBytes(report.availableStorageBytes)}</SettingRow>
  <details>
    <summary>Technical report</summary>
    <pre>{JSON.stringify(report, null, 2)}</pre>
  </details>
{/if}

<style>
  summary {
    min-block-size: var(--target-min-touch);
    align-content: center;
    cursor: pointer;
  }

  pre {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }
</style>
