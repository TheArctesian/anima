import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';

/**
 * jsdom does not load a component's styles, so these read the rule from the source. The layout itself is measured in
 * WebKit and Chromium by the interface review.
 */
function rule(file: string, selector: string): string {
  const text = readFileSync(new URL(file, import.meta.url), 'utf8');
  // The last rule of the selector: a grouped rule above it may end with the same selector.
  const start = text.lastIndexOf(`  ${selector} {`);
  if (start < 0) throw new Error(`no rule ${selector} in ${file}`);
  return text.slice(start, text.indexOf('}', start));
}

it('LOG-009: the device select and its label shrink, so a long device name cannot widen the page', () => {
  for (const selector of ['select', '.select']) {
    const body = rule('../panel/LogFilters.svelte', selector);
    expect(body).toContain('min-inline-size: 0');
    expect(body).toContain('max-inline-size: 100%');
  }
});

it('MOT-007: the panel header has a fixed block size from the token, so the collapsed height is right at first paint', () => {
  expect(rule('../panel/LogsPanel.svelte', 'header')).toContain('block-size: var(--logs-header-size)');
});

it('MOT-007: the panel is pinned to the viewport and starts below the screen, so it is never in the flow', () => {
  const body = rule('../panel/LogsPanel.svelte', '.panel');
  expect(body).toContain('position: fixed');
  expect(body).toContain('translate: 0 100%');
});

it('MOT-007: a long device name is clipped, not run into the message', () => {
  const body = rule('../panel/LogList.svelte', '.device');
  expect(body).toContain('text-overflow: ellipsis');
  expect(body).toContain('min-inline-size: 0');
});

it('MOT-007: the scroll clearance is a constant: the 66 % stop plus the header, never the live height', () => {
  expect(rule('../../devices/screen/DeviceScreen.svelte', '.clearance')).toContain(
    'calc(var(--logs-two-thirds) + var(--logs-header-size))',
  );
});
