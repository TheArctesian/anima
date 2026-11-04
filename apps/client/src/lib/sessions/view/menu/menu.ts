/**
 * The menu item a key moves focus to (WAI-ARIA menu pattern): ArrowDown and ArrowUp step and wrap, Home and End
 * jump to the ends. Undefined for any other key, or when there are no items.
 */
export function menuFocus<T>(key: string, items: readonly T[], active: unknown): T | undefined {
  if (items.length === 0) return undefined;
  const at = items.indexOf(active as T);
  switch (key) {
    case 'ArrowDown':
      return items[(at + 1) % items.length];
    case 'ArrowUp':
      return items[at <= 0 ? items.length - 1 : at - 1];
    case 'Home':
      return items[0];
    case 'End':
      return items.at(-1);
    default:
      return undefined;
  }
}
