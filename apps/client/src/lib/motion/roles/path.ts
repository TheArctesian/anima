/*
 * The path an element takes in and out of a bidirectional transition: a fade with an optional
 * scale, and the part of it that the first direction fixes for the transition's whole life.
 */

export const opacity = (t: number) => `opacity: ${String(t)};`;

/** A fade with a scale from `atZero` when gone to 1 in place; with no scale, a fade only. */
export function scaledFade(atZero: number | undefined, origin: string | undefined) {
  if (atZero === undefined) return opacity;
  const at = origin === undefined ? '' : ` transform-origin: ${origin};`;
  return (t: number) => `${opacity(t)} scale: ${String(atZero + (1 - atZero) * t)};${at}`;
}

/**
 * A value fixed by the first direction of one transition; `own` runs for the first direction
 * only. A reversal mid-flight keeps the value, so the element goes back on the path it came on
 * with no step (MOT-002, MOT-005); Svelte keeps one transition's options until it ends.
 */
export function firstDirection<T>(): (own: () => T) => T {
  let first: { readonly value: T } | undefined;
  return (own) => (first ??= { value: own() }).value;
}
