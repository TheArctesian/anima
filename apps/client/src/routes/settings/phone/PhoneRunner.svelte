<script lang="ts">
  import { tick } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { formatRunnerBytes, readRunnerReport, type RunnerReport } from '$lib/desktop/runner/report';
  import SettingRow from '$lib/settings/SettingRow.svelte';

  let report = $state<RunnerReport>();
  let loading = $state(false);
  let error = $state(false);
  let status = $state('');
  const checked = $derived(report === undefined ? null : new Date(report.observedAt));
  const checkedLabel = $derived(checked === null || Number.isNaN(checked.getTime()) ? null
    : checked.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'medium' }));

  async function refresh(): Promise<void> {
    if (loading) return;
    loading = true;
    status = '';
    error = false;
    await tick();
    try {
      const refreshed = report !== undefined;
      report = await readRunnerReport();
      status = refreshed ? 'Capability report refreshed.' : 'Capability report ready.';
    } catch {
      error = true;
    } finally {
      loading = false;
    }
  }
</script>

<SettingRow label="Capability report" stacked>
  <div class="reading">
    {#if report && checkedLabel !== null}
      <span>Last checked <time datetime={report.observedAt}>{checkedLabel}</time></span>
    {/if}
    <FormButton onclick={refresh} busy={loading}>
      {loading ? 'Reading…' : report ? 'Refresh' : 'Read capabilities'}
    </FormButton>
  </div>
  {#snippet message()}
    <span class="visually-hidden" role="status">{status}</span>
    <p role="alert">{#if error}{report ? 'Could not refresh. Showing the last report.' : 'Could not read this device’s capabilities. Try again.'}{/if}</p>
  {/snippet}
</SettingRow>

{#if report}
  <SettingRow label="Power">{report.charging}</SettingRow>
  <SettingRow label="Thermal state">{report.thermal}</SettingRow>
  {#if report.platform === 'android'}
    <SettingRow label="Available system memory">{formatRunnerBytes(report.availableSystemMemoryBytes)}</SettingRow>
  {:else}
    <SettingRow label="Available app memory">{formatRunnerBytes(report.availableProcessBytes)}</SettingRow>
  {/if}
  <SettingRow label="Available storage">{formatRunnerBytes(report.availableStorageBytes)}</SettingRow>
  <details>
    <summary>Technical report</summary>
    <pre>{JSON.stringify(report, null, 2)}</pre>
  </details>
{/if}

<style>
  .reading {
    display: flex;
    flex-wrap: wrap;
    justify-content: end;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    text-align: end;
  }

  time {
    display: block;
  }

  .visually-hidden {
    position: absolute;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

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
