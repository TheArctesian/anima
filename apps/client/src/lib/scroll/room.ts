/** The height that is left from `top` to the foot of a window of `viewport` px, less `gap`: never below `least`. */
export function roomBelow(top: number, viewport: number, gap: number, least: number): number {
  return Math.max(least, Math.round(viewport - top - gap));
}

/**
 * An attachment for a page whose parts scroll on their own: it sets `--room` on the node to the height from the top of
 * the node to the foot of the window, less `gap` px, so that the page itself does not scroll. The page stylesheet
 * decides where it uses `--room` (a narrow window may not).
 */
export function fillRoom(gap: number, least = 320): (node: HTMLElement) => () => void {
  return (node) => {
    const measure = (): void => {
      const top = node.getBoundingClientRect().top + window.scrollY;
      node.style.setProperty('--room', `${String(roomBelow(top, window.innerHeight, gap, least))}px`);
    };
    measure();
    window.addEventListener('resize', measure);
    const observer = typeof ResizeObserver === 'function' ? new ResizeObserver(measure) : undefined;
    // The strip of sessions or a banner over the page moves the top of the node without a resize of the window.
    const parent = node.closest('main');
    if (parent !== null) observer?.observe(parent);
    return () => {
      window.removeEventListener('resize', measure);
      observer?.disconnect();
    };
  };
}
