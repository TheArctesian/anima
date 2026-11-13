/** Which option a key moves to, or `undefined` when the key is not a way to move. Moving wraps at both ends unless `wrap` is false, which stops at the ends. */
export function movedTo(key: string, current: number, count: number, wrap = true): number | undefined {
  switch (key) {
    case 'ArrowRight':
    case 'ArrowDown':
      return wrap ? (current + 1) % count : Math.min(current + 1, count - 1);
    case 'ArrowLeft':
    case 'ArrowUp':
      return wrap ? (current - 1 + count) % count : Math.max(current - 1, 0);
    case 'Home':
      return 0;
    case 'End':
      return count - 1;
    default:
      return undefined;
  }
}
