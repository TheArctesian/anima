/** The longest link the confirmation shows, in characters; a longer one hides its end from the operator. */
export const LINK_MAX = 2048;

/** Why a link must not open. */
export type LinkRefusal = 'scheme' | 'credentials' | 'too-long';

/** A link that may open, with its parsed address, or the reason it may not. */
export type LinkCheck = { readonly ok: true; readonly address: string } | { readonly ok: false; readonly reason: LinkRefusal };

/** One fixed sentence per reason; none holds the link's own text (LOG-007). */
const REFUSAL_TEXT: Readonly<Record<LinkRefusal, string>> = {
  scheme: 'The device asked to open a link that is not http or https. Anima did not open it.',
  credentials: 'The device asked to open a link with a user name or password in it. Anima did not open it.',
  'too-long': 'The device asked to open a link that is too long to check. Anima did not open it.',
};

export function linkRefusalText(reason: LinkRefusal): string {
  return REFUSAL_TEXT[reason];
}

/**
 * Whether a link may open (K4 §8.7): only `http` and `https`, no user name or password (`https://accounts.google.com:443@evil.example/`
 * shows a host that is not the real one), and at most `LINK_MAX` characters, raw and parsed (percent-encoding can make the parsed
 * address far longer, and it is what the confirmation shows). The address is the parsed form, so the confirmation shows what the
 * browser would really open (an international host name appears in its ASCII form).
 */
export function checkLink(raw: string): LinkCheck {
  if (raw.length > LINK_MAX) return { ok: false, reason: 'too-long' };
  if (!URL.canParse(raw)) return { ok: false, reason: 'scheme' };
  const url = new URL(raw);
  if (url.protocol !== 'http:' && url.protocol !== 'https:') return { ok: false, reason: 'scheme' };
  if (url.username !== '' || url.password !== '') return { ok: false, reason: 'credentials' };
  return url.href.length > LINK_MAX ? { ok: false, reason: 'too-long' } : { ok: true, address: url.href };
}

/** The address a link would open, or `undefined` when it must not open (see `checkLink`). */
export function linkTarget(raw: string): string | undefined {
  const check = checkLink(raw);
  return check.ok ? check.address : undefined;
}

/**
 * Open a confirmed link in the system browser, with no access to this window (`noopener`, `noreferrer`). Plain
 * `window.open` is what the web view does; the desktop shell may route it to the system browser.
 */
export function openLink(url: string): void {
  window.open(url, '_blank', 'noopener,noreferrer');
}
