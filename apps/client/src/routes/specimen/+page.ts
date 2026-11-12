import { error } from '@sveltejs/kit';
import { dev } from '$app/environment';

/**
 * The primitive specimen (UI redesign Phase 3) exists in development, and in a build made with
 * `VITE_ANIMA_SPECIMEN=1` for the gallery and the interface review. A normal build answers 404.
 */
export function load(): void {
  if (!dev && import.meta.env['VITE_ANIMA_SPECIMEN'] !== '1') error(404, 'Not found');
}
