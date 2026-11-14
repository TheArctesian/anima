<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import SplitButton from '$lib/forms/split/SplitButton.svelte';
  import type { SplitChoice } from '$lib/forms/split/split';
  import { resolve } from '$lib/motion/roles/resolve';
  import DetailBar from '$lib/page/DetailBar.svelte';
  import KeyValueList from '$lib/page/KeyValueList.svelte';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import Section from '$lib/page/Section.svelte';
  import SmallCaps from '$lib/page/SmallCaps.svelte';
  import type { Pair } from '$lib/page/pair';
  import Row from '$lib/rows/Row.svelte';
  import RowList from '$lib/rows/RowList.svelte';
  import Tabs from '$lib/tabs/Tabs.svelte';
  import type { TabSpec } from '$lib/tabs/tabs';

  const TABS: readonly TabSpec[] = [
    { id: 'overview', label: 'Overview' },
    { id: 'checks', label: 'Checks', count: 6 },
    { id: 'settings', label: 'Settings' },
  ];
  let selected = $state('overview');
  let chosen = $state('nothing yet');

  const PAIRS: readonly Pair[] = [
    { label: 'Host', value: 'nas.home.arpa', mono: true, icon: 'carbon:server-dns' },
    { label: 'IP address', value: '192.168.178.42', mono: true, icon: 'carbon:ip' },
    { label: 'Operating system', value: 'Debian 13 (trixie)', icon: 'simple-icons:debian' },
    { label: 'Last seen', value: '2 minutes ago', icon: 'carbon:time' },
  ];

  const CONNECT: readonly SplitChoice[] = [
    { id: 'ssh', label: 'SSH', icon: 'carbon:terminal', shortcut: '⌘1' },
    { id: 'vnc', label: 'VNC', icon: 'carbon:screen-map', shortcut: '⌘2' },
    { id: 'wake', label: 'Wake', icon: 'carbon:power' },
  ];
</script>

<!-- UR-7 Phase D: the primitives of the website's language, as the pages will use them. -->
<section class="language" aria-labelledby="specimen-language" {@attach resolve()}>
  <h2 id="specimen-language" class="lane">Language</h2>

  <DetailBar back={{ to: '/devices', label: 'Devices' }} icon="carbon:bare-metal-server" title="nas">
    {#snippet meta()}<span class="up">Up</span><span class="mono">nas.home.arpa</span>{/snippet}
    {#snippet actions()}
      <SplitButton
        label="Connect"
        icon="carbon:terminal"
        tone="primary"
        choices={CONNECT}
        menuLabel="More ways to connect to nas"
        onMain={() => (chosen = 'SSH')}
        onChoose={(choice) => (chosen = choice.label)}
      />
      <FormButton tone="ghost" icon="carbon:overflow-menu-vertical" label="More actions" />
    {/snippet}
    {#snippet reason()}<span>Disk 91% full on /srv. Chosen: {chosen}.</span>{/snippet}
    {#snippet tabs()}<Tabs items={TABS} {selected} label="Device tabs" idPrefix="language" onSelect={(id) => (selected = id)} />{/snippet}
  </DetailBar>

  <PageHeader title="Devices" detail="12 devices · 1 down · 2 with a warning" eyebrow={{ text: 'Your fleet', icon: 'carbon:bare-metal-server' }}>
    {#snippet actions()}<FormButton tone="primary" icon="carbon:add">Add device</FormButton>{/snippet}
  </PageHeader>

  <Section title="Identity" icon="carbon:identification" lede="Where the hub reaches this device.">
    <KeyValueList pairs={PAIRS} />
  </Section>

  <Section title="Devices" icon="carbon:list">
    <div class="caps"><SmallCaps>Needs a look</SmallCaps></div>
    <RowList label="Devices that need a look">
      <Row to="/devices" columns="6rem minmax(0, 1fr) 8rem" edge="down">
        <span class="down">Down</span><span>backup-01</span><span class="quiet">4 min ago</span>
      </Row>
    </RowList>
    <div class="caps"><SmallCaps>Up</SmallCaps></div>
    <RowList label="Devices that are up">
      <Row to="/devices" columns="6rem minmax(0, 1fr) 8rem">
        <span class="up">Up</span><span>nas</span><span class="quiet">99.9%</span>
      </Row>
      <Row to="/devices" columns="6rem minmax(0, 1fr) 8rem">
        <span class="up">Up</span><span>pop-os</span><span class="quiet">100%</span>
      </Row>
    </RowList>
  </Section>
</section>

<style>
  .language {
    display: grid;
    gap: var(--space-10);
    padding-block-end: var(--space-16);
  }

  .lane {
    margin: 0;
    font-size: var(--text-xl);
  }

  .mono {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .up {
    color: var(--status-up-text);
  }

  .down {
    color: var(--status-down-text);
  }

  .quiet {
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }

  .caps {
    padding-block: var(--space-4) var(--space-2);
  }
</style>
