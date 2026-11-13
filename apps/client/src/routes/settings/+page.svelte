<script lang="ts">
  import ApiKeysSection from '$lib/api-keys/ApiKeysSection.svelte';
  import ExtensionsSection from '$lib/extensions/ExtensionsSection.svelte';
  import DefaultsSection from '$lib/checks/defaults/DefaultsSection.svelte';
  import SignOutRow from '$lib/access/SignOutRow.svelte';
  import HubRows from '$lib/hub-address/HubRows.svelte';
  import McpSection from '$lib/mcp/McpSection.svelte';
  import NotificationsSection from '$lib/notifications/NotificationsSection.svelte';
  import PaletteRow from '$lib/palette/PaletteRow.svelte';
  import { capabilities } from '$lib/platform/platform';
  import MotionRow from '$lib/theme/MotionRow.svelte';
  import RetentionSection from '$lib/retention/RetentionSection.svelte';
  import SettingsSection from '$lib/settings/SettingsSection.svelte';
  import ThemeRow from '$lib/theme/ThemeRow.svelte';
  import TypefaceRows from '$lib/typeface/TypefaceRows.svelte';
  import DesktopRows from '$lib/updates/DesktopRows.svelte';
  import VpnSection from '$lib/vpn/VpnSection.svelte';

  // The desktop rows decide each row themselves; the page only leaves out the section when none can show (UPD-006, LOG-008,
  // BLD-020).
  // The Hub section is desktop only: a browser talks only to the hub that served it (CLI-002, D-5).
  const { updates, commandLine, processLog, builds, keyStore, mcpPermissions } = capabilities();
</script>

<div class="page">
  <h1>Settings</h1>

  <!-- One section per subject, in the order of WP-50 F-10; each later section adds one element here. -->
  <SettingsSection id="settings-account" title="Account" icon="nf-md-account_circle" description="Who is signed in to this hub.">
    <SignOutRow />
  </SettingsSection>

  {#if keyStore}
    <SettingsSection id="settings-hub" title="Hub" icon="nf-md-server_network" description="The hub this app talks to.">
      <HubRows />
    </SettingsSection>
  {/if}

  <SettingsSection id="settings-appearance" title="Appearance" icon="nf-md-palette" description="How Anima looks on this device.">
    <ThemeRow />
    <PaletteRow />
    <MotionRow />
    <TypefaceRows />
  </SettingsSection>

  <SettingsSection id="settings-notifications" title="Notifications" icon="nf-md-bell_outline" description="Where the hub sends alerts.">
    <NotificationsSection />
  </SettingsSection>

  <SettingsSection id="settings-check-defaults" title="Default check settings" icon="nf-md-tune" description="New checks start with these settings.">
    <DefaultsSection />
  </SettingsSection>

  <SettingsSection id="settings-retention" title="Data retention" icon="nf-md-database_clock" description="How long the hub keeps each kind of record.">
    <RetentionSection />
  </SettingsSection>

  {#if updates || commandLine || processLog || builds}
    <SettingsSection id="settings-updates" title="Updates" icon="nf-md-update" description="This app's version, its command line tool, its log and its release builds.">
      <DesktopRows />
    </SettingsSection>
  {/if}

  <SettingsSection id="settings-api-keys" title="API keys" icon="nf-md-key_variant" description="Keys for programs that call the hub's GraphQL API.">
    <ApiKeysSection />
  </SettingsSection>

  <SettingsSection id="settings-extensions" title="Extensions" icon="nf-md-folder" description="What Anima does besides devices. Each starts off; a change is saved at once.">
    <ExtensionsSection />
  </SettingsSection>

  <!-- Desktop only: the MCP permissions are a file on this computer, which a browser cannot reach (MCP-023). -->
  {#if mcpPermissions}
    <SettingsSection id="settings-mcp" title="MCP" icon="nf-md-shield_key" description="What coding agents may do through the anima mcp server on this computer.">
      <McpSection />
    </SettingsSection>
  {/if}

  <!-- Last: the picture arrives after the page and has many rows, so nothing below it can move (MOT-007). -->
  <SettingsSection id="settings-vpn" title="VPN" icon="nf-md-lan_connect" description="How the hub reaches each site, and when each VPN key expires.">
    <VpnSection />
  </SettingsSection>
</div>

<style>
  /* One readable column, as the system's own settings window (SET-001). */
  .page {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
    max-inline-size: calc(var(--space-16) * 11);
  }

  /* The title's line height is reserved from tokens, so the font load cannot move it (WP-11 H-7, R-042). */
  h1 {
    margin: 0;
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
    min-block-size: calc(var(--text-2xl) * var(--leading-tight));
  }
</style>
