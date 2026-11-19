<script lang="ts">
  import ApiKeysSection from '$lib/api-keys/ApiKeysSection.svelte';
  import ExtensionsSection from '$lib/extensions/ExtensionsSection.svelte';
  import DefaultsSection from '$lib/checks/defaults/DefaultsSection.svelte';
  import SignOutRow from '$lib/access/SignOutRow.svelte';
  import { hubIsHosted } from '$lib/hosted/mark';
  import { useHub } from '$lib/hub/context';
  import HubRows from '$lib/hub-address/HubRows.svelte';
  import HubLinkRow from '$lib/link/HubLinkRow.svelte';
  import LinkDevicesSection from '$lib/link/LinkDevicesSection.svelte';
  import { desktopLocalHubShell, LocalHubWatch } from '$lib/local-hub/local-hub.svelte';
  import LocalHubSection from '$lib/local-hub/LocalHubSection.svelte';
  import McpSection from '$lib/mcp/McpSection.svelte';
  import NotificationsSection from '$lib/notifications/NotificationsSection.svelte';
  import PaletteRow from '$lib/palette/PaletteRow.svelte';
  import { capabilities } from '$lib/platform/platform';
  import MotionRow from '$lib/theme/MotionRow.svelte';
  import RetentionSection from '$lib/retention/RetentionSection.svelte';
  import SettingsSection from '$lib/settings/SettingsSection.svelte';
  import SourcesSection from '$lib/sources/SourcesSection.svelte';
  import StayAwakeRows from '$lib/stay-awake/StayAwakeRows.svelte';
  import ThemeRow from '$lib/theme/ThemeRow.svelte';
  import TypefaceRows from '$lib/typeface/TypefaceRows.svelte';
  import DesktopRows from '$lib/updates/DesktopRows.svelte';
  import VpnSection from '$lib/vpn/VpnSection.svelte';
  import PageHeader from '$lib/page/PageHeader.svelte';
  import { resolve as entrance } from '$lib/motion/roles/resolve';
  import PageIndex from '$lib/page/contents/PageIndex.svelte';
  import { SECTIONS, indexOf } from './index';
  import { phoneLayout } from '$lib/nav/phone/layout.svelte';
  import PhoneAccount from './phone/PhoneAccount.svelte';
  import PhoneHub from './phone/PhoneHub.svelte';
  import PhoneLeave from './phone/PhoneLeave.svelte';
  import './phone/settings.css';

  // The desktop rows decide each row themselves; the page only leaves out the section when none can show (UPD-006, LOG-008,
  // BLD-020).
  // The Hub section is desktop only: a browser talks only to the hub that served it (CLI-002, D-5).
  const { updates, commandLine, processLog, builds, keyStore, mcpPermissions, stayAwake, localHub } = capabilities();
  // The hub on this computer: its section, and its origin for the Stay awake notice (FRN-006, AWK-005, FRN-008).
  // A hub that Anima.ing hosts has no VPN, so the page has no VPN section for it (R-211).
  const vpnShown = !hubIsHosted(useHub().address.origin);
  const hubState = localHub ? new LocalHubWatch(desktopLocalHubShell) : null;
  $effect(() => hubState?.watch());
  const desktopRows = updates || commandLine || processLog || builds;
  const index = indexOf({ keyStore, stayAwake, desktopRows, mcpPermissions, vpn: vpnShown });
  // The phone layout has the rows of mockup 7 for Account and Hub, and Leave hub at the end.
  const phone = $derived(phoneLayout());
</script>

<div data-settings-page>
<div class="head" {@attach entrance()}>
  <PageHeader title="Settings" eyebrow={{ text: 'This computer and the hub', icon: 'carbon:settings' }} />
</div>

<!-- The index of the sections beside one readable column of them (UR-7, SET-001); on a narrow window the index is a
     row of links above. -->
<div class="settings">
<PageIndex entries={index} label="Settings sections" />
<div class="page">

  <!-- One section per subject, in the order of WP-50 F-10; each later section adds one element here. -->
  <SettingsSection {...SECTIONS.account}>
    {#if phone}<PhoneAccount />{:else}<SignOutRow />{/if}
  </SettingsSection>

  {#if phone}
    <SettingsSection {...SECTIONS.hub}>
      <PhoneHub />
    </SettingsSection>
  {:else if keyStore}
    <SettingsSection {...SECTIONS.hub}>
      <HubRows />
      {#if localHub}<HubLinkRow />{/if}
    </SettingsSection>
  {/if}

  {#if hubState !== null}
    <LocalHubSection section={SECTIONS.localHub} {hubState} />
  {/if}

  <!-- Desktop only: the shell keeps this computer awake (AWK-004). -->
  {#if stayAwake}
    <SettingsSection {...SECTIONS.stayAwake}>
      <StayAwakeRows localHub={hubState?.origin ?? null} />
    </SettingsSection>
  {/if}

  <SettingsSection {...SECTIONS.appearance}>
    <ThemeRow />
    <PaletteRow />
    <MotionRow />
    <TypefaceRows />
  </SettingsSection>

  <SettingsSection {...SECTIONS.notifications}>
    <NotificationsSection />
  </SettingsSection>

  <SettingsSection {...SECTIONS.checkDefaults}>
    <DefaultsSection />
  </SettingsSection>

  <SettingsSection {...SECTIONS.retention}>
    <RetentionSection />
  </SettingsSection>

  {#if desktopRows}
    <SettingsSection {...SECTIONS.updates}>
      <DesktopRows />
    </SettingsSection>
  {/if}

  <LinkDevicesSection section={SECTIONS.linkDevices} />

  <SettingsSection {...SECTIONS.apiKeys}>
    <ApiKeysSection />
  </SettingsSection>

  <SettingsSection {...SECTIONS.extensions}>
    <ExtensionsSection />
  </SettingsSection>

  <SettingsSection {...SECTIONS.integrations}>
    <SourcesSection />
  </SettingsSection>

  <!-- Desktop only: the MCP permissions are a file on this computer, which a browser cannot reach (MCP-023). -->
  {#if mcpPermissions}
    <SettingsSection {...SECTIONS.mcp}>
      <McpSection />
    </SettingsSection>
  {/if}

  <!-- Last: the picture arrives after the page and has many rows, so nothing below it can move (MOT-007). -->
  {#if vpnShown}
    <SettingsSection {...SECTIONS.vpn}>
      <VpnSection />
    </SettingsSection>
  {/if}

  {#if phone && keyStore}
    <SettingsSection {...SECTIONS.leave}>
      <PhoneLeave />
    </SettingsSection>
  {/if}
</div>
</div>
</div>

<style>
  .head {
    margin-block-end: var(--space-8);
  }

  .settings {
    display: grid;
    grid-template-columns: calc(var(--space-16) * 3 + var(--space-4)) minmax(0, calc(var(--space-16) * 11));
    gap: var(--space-12);
    align-items: start;
  }

  .page {
    display: flex;
    flex-direction: column;
    gap: var(--space-10);
    min-inline-size: 0;
  }

  @media (max-width: 1100.98px) {
    .settings {
      grid-template-columns: minmax(0, 1fr);
      gap: var(--space-5);
    }
  }
</style>
