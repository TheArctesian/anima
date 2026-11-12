/** Which ends of a scrolling strip or area have content past them: `start` when scrolled away from the beginning, `end` when more lies ahead. */
export function hiddenEdges(scrollStart: number, clientSize: number, scrollSize: number): { start: boolean; end: boolean } {
  return { start: scrollStart > 0, end: scrollStart + clientSize < scrollSize - 1 };
}

/**
 * An attachment for a vertically scrolling body: it marks `data-more-above` and `data-more-below` while content lies
 * past that edge, so the stylesheet fades the edge (MAT-003, MAT-004) and shows no line.
 */
export function edgeFade(body: HTMLElement): () => void {
  const mark = (): void => {
    const edges = hiddenEdges(body.scrollTop, body.clientHeight, body.scrollHeight);
    body.toggleAttribute('data-more-above', edges.start);
    body.toggleAttribute('data-more-below', edges.end);
  };
  mark();
  body.addEventListener('scroll', mark, { passive: true });
  const observer = typeof ResizeObserver === 'function' ? new ResizeObserver(mark) : undefined;
  observer?.observe(body);
  return () => {
    body.removeEventListener('scroll', mark);
    observer?.disconnect();
  };
}
