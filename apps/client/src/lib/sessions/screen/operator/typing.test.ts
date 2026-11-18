// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { clearAfterTyping } from './typing';

afterEach(() => {
  vi.useRealTimers();
});

function typed(box: HTMLTextAreaElement, text: string, isComposing: boolean): void {
  box.value += text;
  box.dispatchEvent(new InputEvent('input', { data: text, inputType: 'insertText', isComposing }));
}

it('M4-1: the box is empty after a typed character, once the terminal has read it', () => {
  vi.useFakeTimers();
  const box = document.createElement('textarea');
  clearAfterTyping(box);
  typed(box, 'h', false);
  expect(box.value).toBe('h');
  vi.runAllTimers();
  expect(box.value).toBe('');
});

it('M4-1: text the keyboard is still composing stays in the box', () => {
  vi.useFakeTimers();
  const box = document.createElement('textarea');
  clearAfterTyping(box);
  typed(box, 'he', true);
  vi.runAllTimers();
  expect(box.value).toBe('he');
});
