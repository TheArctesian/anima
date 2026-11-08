<script lang="ts">
  import type { SmtpSecurity } from '$lib/hub/generated/graphql';
  import Field from '../Field.svelte';
  import { SMTP_PORT, type ChannelDraft, type Stored } from './draft';
  import type { DraftField } from './input';
  import SecretField from './SecretField.svelte';

  interface Props {
    email: ChannelDraft['email'];
    stored: Stored | null;
    problemOf: (field: DraftField) => string;
  }

  let { email = $bindable(), stored, problemOf }: Props = $props();

  /** A port still at the other mode's standard follows the mode; a port the operator chose stays. */
  function securityChanged(next: SmtpSecurity): void {
    if (email.port === String(SMTP_PORT[email.security])) email.port = String(SMTP_PORT[next]);
    email.security = next;
  }
</script>

<Field label="SMTP host" problem={problemOf('email.host')}>
  {#snippet control(attributes)}
    <input {...attributes} type="text" bind:value={email.host} autocomplete="off" autocapitalize="off" spellcheck="false" />
  {/snippet}
</Field>
<Field label="Security" hint="Implicit TLS (465) or STARTTLS (587); never a plain login." problem="">
  {#snippet control(attributes)}
    <select
      {...attributes}
      value={email.security}
      onchange={(event) => {
        securityChanged(event.currentTarget.value as SmtpSecurity);
      }}
    >
      <option value="STARTTLS">STARTTLS</option>
      <option value="TLS">TLS</option>
    </select>
  {/snippet}
</Field>
<Field label="Port" problem={problemOf('email.port')}>
  {#snippet control(attributes)}
    <input {...attributes} type="text" inputmode="numeric" bind:value={email.port} autocomplete="off" />
  {/snippet}
</Field>
<Field label="User name" hint="Optional." problem="">
  {#snippet control(attributes)}
    <input {...attributes} type="text" bind:value={email.username} autocomplete="off" autocapitalize="off" spellcheck="false" />
  {/snippet}
</Field>
<SecretField
  label="Password"
  shape="hidden"
  bind:value={email.password.value}
  bind:clear={email.password.clear}
  problem=""
  stored={stored?.emailPassword === true}
  removable
  hint="Optional."
/>
<Field label="From" problem={problemOf('email.from')}>
  {#snippet control(attributes)}
    <input {...attributes} type="email" bind:value={email.from} autocomplete="off" spellcheck="false" />
  {/snippet}
</Field>
<Field label="To" hint="Separate addresses with commas." problem={problemOf('email.to')}>
  {#snippet control(attributes)}
    <input {...attributes} type="text" bind:value={email.to} autocomplete="off" spellcheck="false" />
  {/snippet}
</Field>
