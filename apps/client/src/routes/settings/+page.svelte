<script lang="ts">
  import ApiKeysSection from '$lib/api-keys/ApiKeysSection.svelte';
  import DefaultsSection from '$lib/checks/defaults/DefaultsSection.svelte';
  import SignOutRow from '$lib/access/SignOutRow.svelte';
  import { capabilities } from '$lib/platform/platform';
  import MotionRow from '$lib/theme/MotionRow.svelte';
  import RetentionSection from '$lib/retention/RetentionSection.svelte';
  import ThemeRow from '$lib/theme/ThemeRow.svelte';
  import TypefaceRows from '$lib/typeface/TypefaceRows.svelte';
  import DesktopRows from '$lib/updates/DesktopRows.svelte';

  // The desktop rows decide each row themselves; the page only leaves out the heading when none can show (UPD-006, LOG-008).
  const { updates, processLog } = capabilities();
</script>

<h1>Settings</h1>

<!-- One section per subject, in the order of WP-50 F-10; each later section adds one element here. -->
<section aria-labelledby="settings-account">
  <h2 id="settings-account">Account</h2>
  <SignOutRow />
</section>

<section aria-labelledby="settings-appearance">
  <h2 id="settings-appearance">Appearance</h2>
  <ThemeRow />
  <MotionRow />
  <TypefaceRows />
</section>

<section aria-labelledby="settings-check-defaults">
  <h2 id="settings-check-defaults">Default check settings</h2>
  <DefaultsSection />
</section>

<section aria-labelledby="settings-retention">
  <h2 id="settings-retention">Data retention</h2>
  <RetentionSection />
</section>

{#if updates || processLog}
  <section aria-labelledby="settings-updates">
    <h2 id="settings-updates">Updates</h2>
    <DesktopRows />
  </section>
{/if}

<section aria-labelledby="settings-api-keys">
  <h2 id="settings-api-keys">API keys</h2>
  <ApiKeysSection />
</section>

<style>
  /* The title's line height is reserved from tokens, so the font load cannot move it (WP-11 H-7, R-042). */
  h1 {
    margin: 0;
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
    min-block-size: calc(var(--text-2xl) * var(--leading-tight));
  }

  section {
    margin-block-start: var(--space-10);
  }

  /* Each heading's line is reserved the same way, so nothing below it moves when the font arrives (MOT-007). */
  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-bold);
    line-height: var(--leading-tight);
    min-block-size: calc(var(--text-lg) * var(--leading-tight));
  }
</style>
