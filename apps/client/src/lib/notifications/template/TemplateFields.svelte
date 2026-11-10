<script lang="ts">
  import Field from '../Field.svelte';
  import type { NotificationTemplateInput } from '../api';
  import type { TemplateProblems } from './template';

  interface Props {
    template: NotificationTemplateInput;
    problems: TemplateProblems;
    /** The hub's placeholders (D-21), named in the hint. */
    placeholders: readonly string[];
    disabled?: boolean;
  }

  let { template = $bindable(), problems, placeholders, disabled = false }: Props = $props();
</script>

<Field label="Title" hint="Placeholders: {placeholders.join(' ')}" problem={problems.title}>
  {#snippet control(attributes)}
    <input {...attributes} type="text" bind:value={template.title} {disabled} autocomplete="off" />
  {/snippet}
</Field>
<Field label="Body" problem={problems.body}>
  {#snippet control(attributes)}
    <textarea {...attributes} bind:value={template.body} {disabled} rows="3" autocomplete="off"></textarea>
  {/snippet}
</Field>
