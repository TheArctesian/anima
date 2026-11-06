// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { clickedControl, openerOf, takeOpener, watchClicks } from './opener';

afterEach(() => {
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

function page(html: string): HTMLElement {
  const root = document.createElement('div');
  root.tabIndex = 0;
  root.innerHTML = html;
  document.body.append(root);
  return root;
}

const byId = (id: string) => document.getElementById(id) as HTMLElement;

it('Modal: a clicked button that did not take the focus is the opener, not the panel around it', () => {
  const panel = page('<span><button id="delete"><span id="icon">x</span></button></span>');
  const clicked = clickedControl(byId('icon'));

  expect(clicked?.id).toBe('delete');
  expect(openerOf(panel, clicked)?.id).toBe('delete');
  expect(openerOf(document.body, clicked)?.id).toBe('delete');
});

it('Modal: a dialog opened while focus is elsewhere returns the focus there, not to an older click', () => {
  const root = page('<button id="old">Old</button><input id="field" />');

  expect(openerOf(byId('field'), clickedControl(byId('old')))).toBe(byId('field'));
  expect(openerOf(root, null)).toBe(root);
});

it('Modal: a clicked control that left the document is not the opener', () => {
  const root = page('<button id="gone">Gone</button>');
  const clicked = clickedControl(byId('gone'));
  clicked?.remove();

  expect(openerOf(root, clicked)).toBe(root);
  expect(clickedControl(document.body)).toBeNull();
});

it('Modal: the click that opens a dialog is its opener, and only once', () => {
  page('<button id="delete">Delete</button>');
  const stop = watchClicks();

  byId('delete').click();

  expect(takeOpener(document.body)?.id).toBe('delete');
  expect(takeOpener(document.body)).toBe(document.body);
  stop();
});

it('Modal: after a key press an old click is forgotten, so a dialog opened from the keyboard returns focus to the body', () => {
  page('<button id="tab">Checks</button>');
  const stop = watchClicks();
  byId('tab').click();

  document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', bubbles: true }));

  expect(document.activeElement).toBe(document.body);
  expect(takeOpener(document.activeElement)).toBe(document.body);
  stop();
});

it('Modal: every dialog shares one click listener, and it is removed when the last one stops watching', () => {
  const add = vi.spyOn(document, 'addEventListener');
  const remove = vi.spyOn(document, 'removeEventListener');
  const clicks = (spy: typeof add) => spy.mock.calls.filter(([type]) => type === 'click').length;

  const first = watchClicks();
  const second = watchClicks();
  expect(clicks(add)).toBe(1);
  first();
  expect(clicks(remove)).toBe(0);
  second();

  expect(clicks(remove)).toBe(1);
  page('<button id="later">Later</button>');
  byId('later').click();
  expect(takeOpener(document.body)).toBe(document.body);
});
