/**
 * The address a link would open, or `undefined` when it must not open (K4 §8.7): only `http` and `https`. The
 * address is the parsed form, so the confirmation shows what the browser would really open (an international
 * host name appears in its ASCII form), and `javascript:`, `data:`, `file:` and every other scheme are refused.
 */
export function linkTarget(raw: string): string | undefined {
  if (!URL.canParse(raw)) return undefined;
  const url = new URL(raw);
  return url.protocol === 'http:' || url.protocol === 'https:' ? url.href : undefined;
}

/**
 * Open a confirmed link in the system browser, with no access to this window (`noopener`, `noreferrer`). Plain
 * `window.open` is what the web view does; the desktop shell may route it to the system browser.
 */
export function openLink(url: string): void {
  window.open(url, '_blank', 'noopener,noreferrer');
}
