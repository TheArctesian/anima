// @vitest-environment jsdom
import { afterEach, expect, it } from 'vitest';
import { focusableIn, initialFocusIn, trapTab } from './focus';

afterEach(() => {
  document.body.replaceChildren();
});

function dialog(html: string): HTMLElement {
  const root = document.createElement('div');
  root.tabIndex = -1;
  root.innerHTML = html;
  document.body.append(root);
  return root;
}

function tab(root: HTMLElement, shiftKey = false, init: KeyboardEventInit = {}): KeyboardEvent {
  const event = new KeyboardEvent('keydown', { key: 'Tab', shiftKey, cancelable: true, ...init });
  trapTab(event, root);
  return event;
}

const ids = (elements: readonly HTMLElement[]) => elements.map((element) => element.id);

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

it('Modal: hidden, inert, disabled and tabindex -1 elements are not Tab stops', () => {
  const root = dialog(`
    <button id="kept">A</button>
    <div hidden><input id="in-hidden" /></div>
    <div inert><button id="in-inert">B</button></div>
    <fieldset disabled><input id="in-disabled-fieldset" /></fieldset>
    <button id="minus" tabindex="-1">C</button>
    <input type="hidden" id="hidden-input" />
    <a id="link" href="#x">D</a>`);

  expect(ids(focusableIn(root))).toEqual(['kept', 'link']);
});

it('Modal: a radio group is one Tab stop, at its checked button', () => {
  const root = dialog(`
    <input type="radio" name="auth" id="password" />
    <input type="radio" name="auth" id="key" checked />
    <button id="after">Save</button>`);

  expect(ids(focusableIn(root))).toEqual(['key', 'after']);
});

it('Modal: Tab moves the focus to each button in turn, so WebKit does not skip buttons or leave the dialog', () => {
  const root = dialog('<input id="name" /><button id="test">Test</button><button id="cancel">Cancel</button><button id="save">Save</button>');
  document.getElementById('name')?.focus();
  const seen: string[] = [];

  for (let press = 0; press < 4; press += 1) {
    const event = tab(root);
    expect(event.defaultPrevented).toBe(true);
    seen.push(document.activeElement?.id ?? '');
  }

  expect(seen).toEqual(['test', 'cancel', 'save', 'name']);
  tab(root, true);
  expect(document.activeElement?.id).toBe('save');
});

it('Modal: Tab passes over a stop that does not take the focus', () => {
  const root = dialog('<button id="a">A</button><button id="gone">B</button><button id="c">C</button>');
  const gone = document.getElementById('gone') as HTMLElement;
  gone.focus = () => undefined;
  document.getElementById('a')?.focus();

  tab(root);

  expect(document.activeElement?.id).toBe('c');
});

it('Modal: Tab in a dialog with no stops keeps the focus on the dialog', () => {
  const root = dialog('<p>text</p>');
  const outside = document.createElement('button');
  document.body.append(outside);
  outside.focus();

  const event = tab(root);

  expect(event.defaultPrevented).toBe(true);
  expect(document.activeElement).toBe(root);
});

it('Modal: Tab from the radio button the operator arrowed to leaves the group, even when that one is not checked', () => {
  const root = dialog(`
    <button id="before">Before</button>
    <input type="radio" name="auth" id="password" checked />
    <input type="radio" name="auth" id="key" />
    <button id="after">After</button>`);
  document.getElementById('key')?.focus();

  tab(root);

  expect(document.activeElement?.id).toBe('after');
});

it('Modal: with no radio button checked, Tab enters the group at its first and Shift+Tab at its last', () => {
  const root = dialog(`
    <button id="before">Before</button>
    <input type="radio" name="auth" id="password" />
    <input type="radio" name="auth" id="key" />
    <button id="after">After</button>`);
  document.getElementById('before')?.focus();
  tab(root);
  expect(document.activeElement?.id).toBe('password');

  document.getElementById('after')?.focus();
  tab(root, true);
  expect(document.activeElement?.id).toBe('key');
});

it('Modal: radio buttons with one name in two forms are two groups', () => {
  const root = dialog(`
    <form><input type="radio" name="mode" id="first-form" /></form>
    <form><input type="radio" name="mode" id="second-form" /></form>`);

  expect(ids(focusableIn(root))).toEqual(['first-form', 'second-form']);
});

it('Modal: Tab with Ctrl, Alt or Meta is left to the browser and the system', () => {
  const root = dialog('<button id="a">A</button><button id="b">B</button>');
  document.getElementById('a')?.focus();

  for (const modifier of ['ctrlKey', 'altKey', 'metaKey'] as const) {
    const event = tab(root, false, { [modifier]: true });
    expect(event.defaultPrevented).toBe(false);
    expect(document.activeElement?.id).toBe('a');
  }
});
