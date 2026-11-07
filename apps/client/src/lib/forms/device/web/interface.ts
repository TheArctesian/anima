import type { FieldError } from '$lib/hub/error';
import type { OpenIn, WebInterface, WebInterfaceInput } from '$lib/web-interfaces/api';

/** What the operator edits for one web interface (Section 6.3.4). */
export interface WebValues {
  label: string;
  url: string;
  openIn: OpenIn;
}

export const OPEN_IN_CHOICES: readonly { readonly value: OpenIn; readonly text: string }[] = [
  { value: 'ANIMA_TAB', text: 'Anima tab' },
  { value: 'SYSTEM_BROWSER', text: 'System browser' },
];

/** The values of a stored interface, or the defaults for a new one (Anima tab). */
export function webValuesFrom(saved?: WebInterface): WebValues {
  return { label: saved?.label ?? '', url: saved?.url ?? '', openIn: saved?.openIn ?? 'ANIMA_TAB' };
}

/** WEB-001: the URL must be http or https and name a host; the label is optional. */
export function validateWeb(values: WebValues): FieldError[] {
  const text = values.url.trim();
  if (text === '') return [{ field: 'url', message: 'Enter the URL of the web interface.' }];
  const url = URL.canParse(text) ? new URL(text) : null;
  if (url === null || (url.protocol !== 'http:' && url.protocol !== 'https:') || url.hostname === '') {
    return [{ field: 'url', message: 'Enter an http or https URL with a host, for example https://pve.lan:8006.' }];
  }
  return [];
}

/** An empty label is sent as null: the hub then uses the host of the URL. */
export function webInput(values: WebValues): WebInterfaceInput {
  const label = values.label.trim();
  return { label: label === '' ? null : label, url: values.url.trim(), openIn: values.openIn };
}

/** Whether `values` differ from the stored interface (a missing one is a new row, which is always ready to save). */
export function webChanged(values: WebValues, saved?: WebInterface): boolean {
  if (saved === undefined) return true;
  const before = webValuesFrom(saved);
  return values.label.trim() !== before.label || values.url.trim() !== before.url || values.openIn !== before.openIn;
}
