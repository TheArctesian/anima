<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import BackLink from '$lib/page/BackLink.svelte';
  import KeyValueList from '$lib/page/KeyValueList.svelte';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import Section from '$lib/page/Section.svelte';
  import type { Pair } from '$lib/page/pair';
  import Tabs from '$lib/tabs/Tabs.svelte';
  import type { TabSpec } from '$lib/tabs/tabs';

  const TABS: readonly TabSpec[] = [
    { id: 'overview', label: 'Overview' },
    { id: 'checks', label: 'Checks', count: 6 },
    { id: 'sessions', label: 'Sessions', count: 0 },
    { id: 'logs', label: 'Logs' },
    { id: 'settings', label: 'Settings' },
  ];
  let selected = $state('checks');

  const PAIRS: readonly Pair[] = [
    { label: 'Host', value: 'nas.home.arpa', mono: true },
    { label: 'IP address', value: '192.168.178.42', mono: true },
    { label: 'Host key', value: 'SHA256:3c0f1e8b9a7d4c2e6f5a8b1d0c9e7f2a4b6d8c0e1f3a5b7c9d2e4f6a8b0c1d3e', mono: true },
    { label: 'Operating system', value: 'Debian 13 (trixie), kernel 6.12 on aarch64, installed 14 Mar 2025 by the home-lab playbook' },
    { label: 'Last seen', value: '2 minutes ago' },
  ];
</script>

<div class="stack">
  <h3>Page header, back link, tabs</h3>
  <BackLink to="/devices" label="Devices" />
  <PageHeader title="Page header" detail="Document title is set to “Page header – Anima”.">
    {#snippet actions()}<FormButton>Edit</FormButton><FormButton tone="primary">Connect</FormButton>{/snippet}
  </PageHeader>
  <Tabs items={TABS} {selected} label="Specimen tabs" idPrefix="specimen" onSelect={(id) => (selected = id)} />

  <h3>Section: with footer, with action, without footer</h3>
  <Section title="Account" lede="Your sign-in name and password.">
    {#snippet action()}<FormButton>Reset</FormButton>{/snippet}
    <p class="body">Fields go here. The card is sized by its content.</p>
    {#snippet footer()}<span class="state">Unsaved changes</span><FormButton tone="primary">Save changes</FormButton>{/snippet}
  </Section>
  <Section title="Device facts" flush>
    <div class="pad"><KeyValueList pairs={PAIRS} /></div>
  </Section>
  <Section title="Right-aligned values">
    <KeyValueList pairs={PAIRS.slice(0, 2)} align="end" />
  </Section>
</div>

<style>
  .stack {
    display: grid;
    gap: var(--space-4);
  }

  h3 {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-md);
  }

  .body {
    margin: 0;
  }

  .state {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .pad {
    padding-inline: var(--space-4);
  }
</style>
