import type { Attachment } from 'svelte/attachments';

/**
 * DEV-021: gives the group `title` as its tooltip and marks it `data-overflowing` while its content is wider than
 * its box, measured by a `ResizeObserver` on the group and each item, so the end fade shows only when text is cut.
 * Without `ResizeObserver` the group is measured once.
 */
export function overflowFade(title: string): Attachment<HTMLElement> {
  return (group) => {
    group.title = title;
    const measure = () => {
      group.toggleAttribute('data-overflowing', group.scrollWidth > group.clientWidth);
    };
    measure();
    if (typeof ResizeObserver !== 'function') return;
    const observer = new ResizeObserver(measure);
    observer.observe(group);
    for (const item of group.children) observer.observe(item);
    return () => {
      observer.disconnect();
    };
  };
}
