// @vitest-environment jsdom
import { afterEach, expect, it } from 'vitest';
import { initialFocusIn } from './focus';

afterEach(() => {
  document.body.replaceChildren();
});

function dialog(html: string): HTMLElement {
  const root = document.createElement('div');
  root.innerHTML = html;
  document.body.append(root);
  return root;
}

it('Modal: the element marked data-autofocus gets the focus even when it is not the first', () => {
  const root = dialog('<button id="first">A</button><button id="second" data-autofocus>B</button>');

  expect(initialFocusIn(root).id).toBe('second');
});

it('Modal: without a marked element the first reachable one gets the focus, and a disabled one is skipped', () => {
  const root = dialog('<button disabled>A</button><button id="second">B</button>');

  expect(initialFocusIn(root).id).toBe('second');
});

it('Modal: a dialog with nothing to focus takes the focus itself', () => {
  const root = dialog('<p>text</p>');

  expect(initialFocusIn(root)).toBe(root);
});
