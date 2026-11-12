<script lang="ts">
  import Section from '$lib/page/Section.svelte';
  import Table from '$lib/tables/Table.svelte';
  import { dayLabel, type Column, type Severity } from '$lib/tables/table';

  interface Entry {
    readonly id: string;
    readonly at: Date;
    readonly level: Severity;
    readonly device: string;
    readonly source: string;
    readonly message: string;
    readonly duration: string;
    readonly count: string;
  }

  const NOW = new Date();
  const ago = (hours: number): Date => new Date(NOW.getTime() - hours * 3_600_000);
  const ENTRIES: readonly Entry[] = [
    { id: '1', at: ago(0.2), level: 'error', device: 'nas', source: 'Check', message: 'TLS certificate nas.home.arpa expires in 6 days and the chain served by the device is incomplete', duration: '1.2 s', count: '3' },
    { id: '2', at: ago(1), level: 'warning', device: 'router', source: 'Session', message: 'SSH host key changed', duration: '14 ms', count: '1' },
    { id: '3', at: ago(30), level: 'info', device: 'switch', source: 'Check', message: 'Ping recovered', duration: '0.4 s', count: '12' },
    { id: '4', at: ago(80), level: 'info', device: 'proxy', source: 'Hub', message: 'Backup finished', duration: '2 min 8 s', count: '1' },
  ];

  const COLUMNS: Column<Entry>[] = [
    { key: 'level', header: 'Severity', kind: 'severity', severity: (row) => row.level, min: 6, max: 7 },
    { key: 'time', header: 'Time', kind: 'number', value: (row) => row.at.toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' }), min: 4, max: 5 },
    { key: 'device', header: 'Device', value: (row) => row.device, min: 6, max: 9 },
    { key: 'source', header: 'Source', value: (row) => row.source, min: 5, max: 7 },
    { key: 'message', header: 'Message', value: (row) => row.message, min: 14, flex: true },
    { key: 'duration', header: 'Duration', kind: 'number', value: (row) => row.duration, min: 6, max: 8 },
    { key: 'count', header: 'Count', kind: 'number', value: (row) => row.count, min: 4, max: 5 },
  ];
  const day = (row: Entry): string => dayLabel(row.at, NOW);
</script>

<div class="stack">
  <h3>Table: comfortable with day grouping and a truncated cell; dense without grouping</h3>
  <Section title="Events" lede="Seven columns, no horizontal scroll at 1280 px." flush>
    <Table columns={COLUMNS} rows={ENTRIES} rowKey={(row) => row.id} caption="Events, comfortable rows, grouped by day" groupOf={day} />
  </Section>
  <Section title="Events, dense" flush>
    <Table columns={COLUMNS} rows={ENTRIES} rowKey={(row) => row.id} caption="Events, dense rows" density="dense" />
  </Section>
</div>

<style>
  .stack {
    display: grid;
    gap: var(--space-3);
  }

  h3 {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-md);
  }
</style>
