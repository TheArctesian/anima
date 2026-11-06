import { waitForFirstPaintFonts } from '$lib/typeface/first-paint';
import { storedTypefaces } from '$lib/typeface/typeface';

export const prerender = false;
export const ssr = false;

/**
 * The first render waits for the chosen fonts, so the text is laid out in its final font and no later swap moves the
 * page (MOT-007: CLS 0). A typeface chosen later in Settings is not waited for: it applies after the first render.
 */
export async function load(): Promise<void> {
  await waitForFirstPaintFonts(storedTypefaces(localStorage), 'fonts' in document ? document.fonts : undefined);
}
