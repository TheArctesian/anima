// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { placeNear } from './placement';

afterEach(() => {
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

function setup(layout: { width: number; height: number }, scaled: { width: number; height: number }) {
  const button = document.createElement('button');
  const panel = document.createElement('div');
  document.body.append(button, panel);
  vi.spyOn(document.documentElement, 'clientWidth', 'get').mockReturnValue(1100);
  vi.spyOn(document.documentElement, 'clientHeight', 'get').mockReturnValue(600);
  vi.spyOn(panel, 'offsetWidth', 'get').mockReturnValue(layout.width);
  vi.spyOn(panel, 'offsetHeight', 'get').mockReturnValue(layout.height);
  vi.spyOn(panel, 'getBoundingClientRect').mockReturnValue(new DOMRect(0, 0, scaled.width, scaled.height));
  vi.spyOn(button, 'getBoundingClientRect').mockReturnValue(new DOMRect(1000, 100, 60, 30));
  panel.style.paddingInlineStart = '8px';
  return { button, panel };
}

it('R-079: a panel still scaled by its entry transition is clamped by its layout width, not the smaller scaled one', () => {
  const { button, panel } = setup({ width: 320, height: 60 }, { width: 307, height: 58 });
  placeNear(() => button)(panel);
  expect(Number.parseFloat(panel.style.insetInlineStart) + 320).toBe(1092);
});

it('R-079: an earlier position is reset before the panel is measured', () => {
  const { button, panel } = setup({ width: 200, height: 60 }, { width: 200, height: 60 });
  panel.style.insetInlineStart = '500px';
  let atMeasure = '';
  vi.spyOn(panel, 'offsetWidth', 'get').mockImplementation(() => {
    atMeasure = panel.style.insetInlineStart;
    return 200;
  });
  placeNear(() => button)(panel);
  expect(atMeasure).toBe('0px');
});
