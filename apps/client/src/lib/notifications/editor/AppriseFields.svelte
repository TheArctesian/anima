<script lang="ts">
  import Field from '../Field.svelte';
  import type { ChannelDraft, Stored } from './draft';
  import type { DraftField } from './input';
  import SecretField from './SecretField.svelte';

  interface Props {
    apprise: ChannelDraft['apprise'];
    stored: Stored | null;
    problemOf: (field: DraftField) => string;
  }

  let { apprise = $bindable(), stored, problemOf }: Props = $props();
</script>

<Field label="Apprise server" hint="The Apprise API, for example https://apprise.example.com." problem={problemOf('apprise.server')}>
  {#snippet control(attributes)}
    <input {...attributes} type="url" bind:value={apprise.server} autocomplete="off" autocapitalize="off" spellcheck="false" />
  {/snippet}
</Field>
<Field label="Send with" problem="">
  {#snippet control(attributes)}
    <select {...attributes} bind:value={apprise.mode}>
      <option value="STATELESS_URLS">Apprise URLs</option>
      <option value="CONFIG_KEY">A configuration key on the server</option>
    </select>
  {/snippet}
</Field>
{#if apprise.mode === 'STATELESS_URLS'}
  <SecretField
    label="Apprise URLs"
    shape="lines"
    bind:value={apprise.urls}
    problem={problemOf('apprise.urls')}
    stored={stored?.appriseMode === 'STATELESS_URLS'}
    hint="One per line."
  />
{:else}
  <SecretField label="Configuration key" shape="hidden" bind:value={apprise.key} problem={problemOf('apprise.key')} stored={stored?.appriseMode === 'CONFIG_KEY'} />
{/if}
<Field label="Tags" hint="Optional. Separate tags with commas." problem="">
  {#snippet control(attributes)}
    <input {...attributes} type="text" bind:value={apprise.tags} autocomplete="off" spellcheck="false" />
  {/snippet}
</Field>
