<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import { resolve } from '$lib/motion/roles/resolve';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import Section from '$lib/page/Section.svelte';
  import ContainersSection from './containers/ContainersSection.svelte';
  import ControlsSection from './controls/ControlsSection.svelte';
  import LanguageSection from './language/LanguageSection.svelte';
  import DevicesSection from './devices/DevicesSection.svelte';
  import SettingsLane from './devices/SettingsLane.svelte';
  import Corners from './tokens/Corners.svelte';
  import IconGrid from './tokens/IconGrid.svelte';
  import ScreenControls from './tokens/ScreenControls.svelte';
  import Swatches from './tokens/Swatches.svelte';
  import TypeRows from './tokens/TypeRows.svelte';

  let entrance = $state(0);
</script>

<!-- The specimen (UR-7): first the mockup's Specimen screen, section for section, then every component lane. -->
<div class="specimen">
  {#key entrance}
    <div class="intro" {@attach resolve()}>
      <PageHeader title="Specimen" detail="The design language, D1 to D18" eyebrow={{ text: 'Tokens and primitives', icon: 'carbon:color-palette' }} />
    </div>
  {/key}
  <Section title="Type" icon="carbon:text-font"><TypeRows /></Section>
  <Section title="Colour" icon="carbon:color-palette"><Swatches /></Section>
  <Section title="Icons · Carbon and Simple Icons" icon="carbon:carbon"><IconGrid /></Section>
  <Section title="Controls" icon="carbon:cursor-1"><ScreenControls /></Section>
  <Section title="Corners · 2, 4, 4 px" icon="carbon:corner"><Corners /></Section>
  <Section title="Motion · resolve" icon="carbon:movement">
    <div><FormButton icon="carbon:renew" onclick={() => (entrance += 1)}>Replay the entrance</FormButton></div>
  </Section>
  <Section title="Components" icon="carbon:catalog">
    <div class="lanes">
      <LanguageSection />
      <DevicesSection />
      <SettingsLane />
      <ControlsSection />
      <ContainersSection />
    </div>
  </Section>
</div>

<style>
  .specimen {
    display: grid;
    gap: var(--space-10);
    max-inline-size: var(--content-page);
    padding-block-end: var(--space-16);
  }

  .intro {
    padding-block-start: var(--space-3);
  }

  .lanes {
    display: grid;
    gap: var(--space-8);
  }
</style>
