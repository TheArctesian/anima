import { panelPosition } from './actions';

/** Puts a menu panel under `anchor`, or above it, inside the window; the panel's padding is the margin from the edge. */
export function placeNear(anchor: () => HTMLElement | undefined) {
  return (panel: HTMLElement): void => {
    const button = anchor();
    if (button === undefined) return;
    // Measured from the start edge, so an earlier position cannot squeeze the panel narrower than its text needs.
    panel.style.insetInlineStart = '0px';
    const style = getComputedStyle(panel);
    const margin = Number.parseFloat(style.paddingInlineStart) || 0;
    // The panel is fixed, so its end margin moves nothing: it only carries the gap to the button.
    const gap = Number.parseFloat(style.marginBlockEnd) || 0;
    // The layout size: `getBoundingClientRect` follows the popover's entry scale and would measure it too small.
    const position = panelPosition(
      button.getBoundingClientRect(),
      { width: panel.offsetWidth, height: panel.offsetHeight },
      { width: document.documentElement.clientWidth, height: document.documentElement.clientHeight },
      { margin, gap },
    );
    panel.style.insetInlineStart = `${String(position.left)}px`;
    panel.style.insetBlockStart = `${String(position.top)}px`;
  };
}
