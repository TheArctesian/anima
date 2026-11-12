// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import BackLink from './BackLink.svelte';
import KeyValueList from './KeyValueList.svelte';
import PageHeader from './PageHeader.svelte';
import Section from './Section.svelte';
import { pageTitle } from './title';

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
  document.title = '';
});

const text = (html: string) => createRawSnippet(() => ({ render: () => html }));

it('NAV-010: the page header draws the h1 and sets the document title to "<page> – Anima"', () => {
  app = mount(PageHeader, { target: document.body, props: { title: 'Devices', detail: '12 devices', actions: text('<button>Add device</button>') } });
  flushSync();
  expect(document.querySelector('h1')?.textContent).toBe('Devices');
  expect(document.title).toBe('Devices – Anima');
  expect(pageTitle('Settings')).toBe('Settings – Anima');
  expect(document.querySelector('header p')?.textContent).toBe('12 devices');
  expect(document.querySelector('header button')?.textContent).toBe('Add device');
});

it('DEV-020: the back link is a link with a chevron and the word', () => {
  app = mount(BackLink, { target: document.body, props: { to: '/devices', label: 'Devices' } });
  const link = document.querySelector('a');
  expect(link?.getAttribute('href')).toBe('/devices');
  expect(link?.textContent.replace(/\p{Co}/gu, '').trim()).toBe('Devices');
  expect(link?.querySelector('.icon')).not.toBeNull();
});

it('UI-027: a section is named by its title, shows a lede, an action and a footer only when given', () => {
  app = mount(Section, { target: document.body, props: { title: 'Account', children: text('<p>Body</p>') } });
  expect(document.querySelector('section')?.getAttribute('aria-labelledby')).toBe(document.querySelector('h2')?.id);
  expect(document.querySelector('footer')).toBeNull();
  expect(document.querySelector('header p')).toBeNull();
});

it('SET-005: a section footer holds the save button and its state text', async () => {
  await unmount(app ?? {});
  app = mount(Section, {
    target: document.body,
    props: { title: 'Account', lede: 'Your sign-in', footer: text('<div><button>Save changes</button><span>Saved</span></div>'), children: text('<p>Body</p>') },
  });
  expect(document.querySelector('footer')?.textContent).toBe('Save changesSaved');
  expect(document.querySelector('section')?.getAttribute('aria-describedby')).toBe(document.querySelector('header p')?.id);
});

it('O6: a key-value list is a definition list of label and value, mono values marked, with a copy slot', () => {
  app = mount(KeyValueList, {
    target: document.body,
    props: {
      pairs: [
        { label: 'Host', value: 'nas.lan' },
        { label: 'Fingerprint', value: 'SHA256:abcdef', mono: true, copy: text('<button>Copy</button>') },
      ],
      align: 'end',
    },
  });
  expect([...document.querySelectorAll('dt')].map((term) => term.textContent)).toEqual(['Host', 'Fingerprint']);
  const values = [...document.querySelectorAll('dd')];
  expect(values[0]?.classList.contains('mono')).toBe(false);
  expect(values[1]?.classList.contains('mono')).toBe(true);
  expect(values[1]?.querySelector('button')?.textContent).toBe('Copy');
  expect(document.querySelector('dl')?.classList.contains('end')).toBe(true);
});
