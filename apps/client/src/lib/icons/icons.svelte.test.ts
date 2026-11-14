// @vitest-environment jsdom
import { mount, unmount } from 'svelte';
import { describe, expect, it } from 'vitest';
import Icon from './Icon.svelte';
import { iconShape, type IconName } from './icons';
import { ICON_SHAPES } from './shapes';

const names = Object.keys(ICON_SHAPES) as IconName[];

describe('icon table (UI-014, UI-016)', () => {
  it('names every icon by its Iconify id, Carbon or Simple Icons', () => {
    for (const name of names) expect(name).toMatch(/^(carbon|simple-icons):[a-z0-9]+(-[a-z0-9]+)*$/);
  });

  it('gives every icon a square view box and at least one shape', () => {
    for (const name of names) {
      const shape = iconShape(name);
      expect(shape.width, name).toBe(shape.height);
      expect(shape.parts.length, name).toBeGreaterThan(0);
    }
  });
});

describe('Icon', () => {
  it('UI-015: draws an svg hidden from assistive technology, in the text colour, with the icon’s shapes', () => {
    const app = mount(Icon, { target: document.body, props: { name: 'carbon:chip', size: 'sm' } });
    const svg = document.body.querySelector('svg');
    expect(svg?.getAttribute('aria-hidden')).toBe('true');
    expect(svg?.getAttribute('viewBox')).toBe('0 0 32 32');
    expect(svg?.classList.contains('icon-sm')).toBe(true);
    expect(svg?.querySelectorAll('path')).toHaveLength(iconShape('carbon:chip').parts.length);
    expect(svg?.querySelector('path')?.namespaceURI).toBe('http://www.w3.org/2000/svg');
    void unmount(app);
  });
});
