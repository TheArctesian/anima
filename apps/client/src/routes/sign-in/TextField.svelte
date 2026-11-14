<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';

  interface Props {
    id: string;
    name: string;
    label: string;
    /** Leads the field inside its box, in the tertiary colour (UR-7, D13). */
    icon?: IconName | undefined;
    value: string;
    error?: string | undefined;
    type?: 'text' | 'password';
    autocomplete: 'username' | 'current-password' | 'one-time-code' | 'off';
    /** Digits only: the numeric keyboard, a short field and tabular figures. */
    code?: boolean;
    input?: HTMLInputElement | undefined;
    oninput?: (event: Event & { currentTarget: HTMLInputElement }) => void;
    onkeydown?: (event: KeyboardEvent) => void;
  }

  // `input` is bound by the page so it can move focus to the first invalid field.
  let { id, name, label, icon, value = $bindable(), error, type = 'text', autocomplete, code = false, input = $bindable(), oninput, onkeydown }: Props = $props();
</script>

<div class="field">
  <label for={id}>{label}</label>
  <div class="box" class:code>
    {#if icon !== undefined}<Icon name={icon} size="sm" />{/if}
    <input
      {id}
      {name}
      {type}
      {autocomplete}
      inputmode={code ? 'numeric' : undefined}
      maxlength={code ? 6 : undefined}
      placeholder={code ? '000000' : undefined}
      bind:value
      bind:this={input}
      aria-describedby="{id}-problem"
      aria-invalid={error === undefined ? undefined : 'true'}
      {oninput}
      {onkeydown}
    />
  </div>
  <!-- Always rendered, so aria-describedby always names an element; empty until the field has a problem (UI-030). -->
  <p id="{id}-problem" class="problem" hidden={error === undefined}>
    {#if error !== undefined}<span class="mark"><Icon name="carbon:warning" size="sm" /></span>{error}{/if}
  </p>
</div>

<style>
  .field {
    display: grid;
    gap: var(--space-1);
  }

  label {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  /* The box carries the border, so the icon sits inside the field and the whole box shows focus. */
  .box {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    block-size: var(--control-md);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-base);
    color: var(--text-tertiary);
  }

  .box:has(input[aria-invalid='true']) {
    border-color: var(--danger);
  }

  .box:has(input:focus-visible) {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  input {
    flex: 1;
    min-inline-size: 0;
    block-size: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    outline: none;
  }

  input::placeholder {
    color: var(--text-tertiary);
  }

  /* Six digits wide: the monospace face gives every digit the same width, so the box fits the code and no more. */
  .code {
    inline-size: calc(6ch + 6 * var(--space-1) + 2 * var(--space-3) + 2 * var(--border-thin));
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    letter-spacing: var(--space-1);
  }

  .problem {
    display: flex;
    gap: var(--space-1);
    margin: 0;
    color: var(--status-down-text);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .problem[hidden] {
    display: none;
  }

  .mark {
    line-height: var(--leading-tight);
  }
</style>
