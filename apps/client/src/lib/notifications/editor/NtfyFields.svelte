<script lang="ts">
  import Field from '../Field.svelte';
  import type { ChannelDraft, Stored } from './draft';
  import type { DraftField } from './input';
  import SecretField from './SecretField.svelte';

  interface Props {
    ntfy: ChannelDraft['ntfy'];
    stored: Stored | null;
    problemOf: (field: DraftField) => string;
  }

  let { ntfy = $bindable(), stored, problemOf }: Props = $props();

  const PRIORITIES = [
    { value: 1, label: '1 (min)' },
    { value: 2, label: '2 (low)' },
    { value: 3, label: '3 (default)' },
    { value: 4, label: '4 (high)' },
    { value: 5, label: '5 (max)' },
  ];
</script>

<Field label="Server" hint="For example https://ntfy.sh." problem={problemOf('ntfy.server')}>
  {#snippet control(attributes)}
    <input {...attributes} type="url" bind:value={ntfy.server} autocomplete="off" autocapitalize="off" spellcheck="false" />
  {/snippet}
</Field>
<!-- On a public server the topic is the credential, so it is a secret like the token (D-17). -->
<SecretField label="Topic" bind:value={ntfy.topic} problem={problemOf('ntfy.topic')} stored={stored !== null} />
<SecretField
  label="Access token"
  shape="hidden"
  bind:value={ntfy.token.value}
  bind:clear={ntfy.token.clear}
  problem=""
  stored={stored?.ntfyToken === true}
  removable
  hint="Optional."
/>
<Field label="Priority" problem="">
  {#snippet control(attributes)}
    <select {...attributes} bind:value={ntfy.priority}>
      {#each PRIORITIES as priority (priority.value)}
        <option value={priority.value}>{priority.label}</option>
      {/each}
    </select>
  {/snippet}
</Field>
