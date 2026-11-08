// CR-14: a channel that moves to a new target needs its stored secrets again; URLs are https only; a server is an
// origin only; the Apprise mode and header list rules. Pure: the draft in, the input or the refusals out.
import { describe, expect, it } from 'vitest';
import type { NotificationChannel } from '../api';
import { CONFIG_KEY_CHANNEL, DIRTY_SERVER_CHANNEL, FAKE_SECRET, fixtureChannel, FIXTURE_SETTINGS } from '../notifications.fixture';
import { savedDraft, storedOf, type ChannelDraft } from './draft';
import { AGAIN, AGAIN_OR_REMOVE, APPRISE_URLS_WRONG, checkDraft, HEADERS_AGAIN, SERVER_WRONG, URL_WRONG } from './input';

type DraftCheck = ReturnType<typeof checkDraft>;

const { placeholders, template } = FIXTURE_SETTINGS;

function edited(channel: NotificationChannel, edit: (draft: ChannelDraft) => void = () => undefined): DraftCheck {
  const draft = savedDraft(channel, template);
  edit(draft);
  return checkDraft(draft, storedOf(channel), placeholders);
}

const ntfy = fixtureChannel('NTFY');
const webhook = fixtureChannel('WEBHOOK');
const email = fixtureChannel('EMAIL');
const apprise = fixtureChannel('APPRISE');

function refusals(check: DraftCheck): [string, string][] {
  return check.ok ? [] : check.problems.map((problem) => [problem.field, problem.message]);
}

function settingsOf(check: DraftCheck): unknown {
  if (!check.ok) throw new Error(`refused: ${check.problems.map((problem) => problem.field).join(', ')}`);
  return check.input.settings;
}

describe('a new ntfy or webhook target (CR-14.1)', () => {
  it('CR-14.1, D-17: a new ntfy server with the stored topic and token untouched is refused', () => {
    const check = edited(ntfy, (draft) => {
      draft.ntfy.server = 'https://ntfy.attacker.example';
    });
    expect(refusals(check)).toEqual([['ntfy.topic', AGAIN], ['ntfy.token', AGAIN_OR_REMOVE]]);
  });

  it('CR-14.1: a new ntfy server with the topic typed again and the token typed or removed is accepted', () => {
    const typed = edited(ntfy, (draft) => {
      draft.ntfy.server = 'https://ntfy.other.example';
      draft.ntfy.topic = 'FAKE_SECRET_topic';
      draft.ntfy.token.value = FAKE_SECRET;
    });
    expect(settingsOf(typed)).toEqual({ ntfy: { server: 'https://ntfy.other.example', topic: 'FAKE_SECRET_topic', token: FAKE_SECRET, priority: 3 } });
    const removed = edited(ntfy, (draft) => {
      draft.ntfy.server = 'https://ntfy.other.example';
      draft.ntfy.topic = 'FAKE_SECRET_topic';
      draft.ntfy.token.clear = true;
    });
    expect(settingsOf(removed)).toMatchObject({ ntfy: { token: '' } });
  });

  it('CR-14.1: a new ntfy server with no stored token needs only the topic typed again', () => {
    const channel = { ...ntfy, settings: { ...ntfy.settings, tokenStored: false } } as NotificationChannel;
    const check = edited(channel, (draft) => {
      draft.ntfy.server = 'https://ntfy.other.example';
      draft.ntfy.topic = 'FAKE_SECRET_topic';
    });
    expect(settingsOf(check)).toMatchObject({ ntfy: { server: 'https://ntfy.other.example', topic: 'FAKE_SECRET_topic', token: null } });
  });

  it('CR-14.1: a webhook URL at a new origin needs the stored headers replaced or removed; at the same origin it does not', () => {
    const away = (edit: (draft: ChannelDraft) => void) =>
      edited(webhook, (draft) => {
        draft.webhook.url = `https://hooks.attacker.example/${FAKE_SECRET}`;
        edit(draft);
      });
    expect(refusals(away(() => undefined))).toEqual([['webhook.headers', HEADERS_AGAIN]]);
    const replaced = away((draft) => {
      draft.webhook.replaceHeaders = true;
      draft.webhook.headers = [{ name: 'X-Anima-Source', value: FAKE_SECRET }];
    });
    expect(settingsOf(replaced)).toMatchObject({ webhook: { headers: [{ name: 'X-Anima-Source', value: FAKE_SECRET }] } });
    const removed = away((draft) => {
      draft.webhook.replaceHeaders = true;
    });
    expect(settingsOf(removed)).toMatchObject({ webhook: { headers: [] } });
    const home = edited(webhook, (draft) => {
      draft.webhook.url = `https://hooks.example.net/other/${FAKE_SECRET}`;
    });
    expect(settingsOf(home)).toMatchObject({ webhook: { headers: null } });
  });

});

describe('a new mail or Apprise target, and the same target written again (CR-14.1)', () => {
  it.each([
    ['host', (draft: ChannelDraft) => { draft.email.host = 'smtp.attacker.example'; }],
    ['port', (draft: ChannelDraft) => { draft.email.security = 'TLS'; draft.email.port = '465'; }],
  ])('CR-14.1: a new SMTP %s needs the stored password typed or removed', (_, move) => {
    expect(refusals(edited(email, move))).toEqual([['email.password', AGAIN_OR_REMOVE]]);
    const typed = edited(email, (draft) => {
      move(draft);
      draft.email.password.value = FAKE_SECRET;
    });
    expect(settingsOf(typed)).toMatchObject({ email: { password: FAKE_SECRET } });
    const removed = edited(email, (draft) => {
      move(draft);
      draft.email.password.clear = true;
    });
    expect(settingsOf(removed)).toMatchObject({ email: { password: '' } });
  });

  it('CR-14.1: a new Apprise server needs the URLs of its mode typed again', () => {
    const check = edited(apprise, (draft) => {
      draft.apprise.server = 'https://apprise.attacker.example';
    });
    expect(refusals(check)).toEqual([['apprise.urls', APPRISE_URLS_WRONG]]);
    const typed = edited(apprise, (draft) => {
      draft.apprise.server = 'https://apprise.other.example';
      draft.apprise.urls = `discord://${FAKE_SECRET}`;
    });
    expect(settingsOf(typed)).toMatchObject({ apprise: { server: 'https://apprise.other.example', urls: [`discord://${FAKE_SECRET}`], key: null } });
  });

  it('CR-14.1: the same target, written with a trailing slash or capitals, is not a new one', () => {
    expect(settingsOf(edited(email, (draft) => { draft.email.host = 'MAIL.example.net'; }))).toMatchObject({ email: { host: 'MAIL.example.net', password: null } });
    const check = edited(ntfy, (draft) => {
      draft.ntfy.server = 'https://NTFY.example.net/';
    });
    expect(settingsOf(check)).toMatchObject({ ntfy: { server: 'https://ntfy.example.net', topic: null, token: null } });
  });
});

describe('servers and URLs (CR-14.6, CR-14.7)', () => {
  it('CR-14.6: http is refused for the webhook URL and the ntfy and Apprise servers', () => {
    expect(refusals(edited(webhook, (draft) => { draft.webhook.url = `http://hooks.example.net/${FAKE_SECRET}`; }))).toEqual([['webhook.url', URL_WRONG]]);
    expect(refusals(edited(ntfy, (draft) => { draft.ntfy.server = 'http://ntfy.example.net'; })).at(0)).toEqual(['ntfy.server', SERVER_WRONG]);
    expect(refusals(edited(apprise, (draft) => { draft.apprise.server = 'http://apprise.example.net'; })).at(0)).toEqual(['apprise.server', SERVER_WRONG]);
  });

  it.each([
    `https://ops:${FAKE_SECRET}@ntfy.example.net`,
    'https://@ntfy.example.net',
    'https://ntfy.example.net/path',
    'https://ntfy.example.net?q=1',
    'https://ntfy.example.net/?',
    'https://ntfy.example.net#top',
    'https://ntfy.example.net\\@other.example',
  ])('CR-14.7: the server %s is not an origin only, and is refused', (server) => {
    const check = edited(ntfy, (draft) => {
      draft.ntfy.server = server;
    });
    expect(refusals(check).at(0)).toEqual(['ntfy.server', SERVER_WRONG]);
    expect(JSON.stringify(refusals(check))).not.toContain(FAKE_SECRET);
  });

  it('CR-14.7, F-8: a server the hub holds with a user name and password shows as its origin only, and counts as a new target', () => {
    const draft = savedDraft(DIRTY_SERVER_CHANNEL, template);
    expect(draft.ntfy.server).toBe('https://ntfy.example.net');
    expect(JSON.stringify(draft)).not.toContain(FAKE_SECRET);
    expect(refusals(edited(DIRTY_SERVER_CHANNEL))).toEqual([['ntfy.topic', AGAIN], ['ntfy.token', AGAIN_OR_REMOVE]]);
  });
});

describe('Apprise modes and header lists (CR-14.2, CR-14.3)', () => {
  it('CR-14.2: an untouched CONFIG_KEY channel keeps its key with null and sends no URLs', () => {
    expect(settingsOf(edited(CONFIG_KEY_CHANNEL))).toEqual({ apprise: { server: 'https://apprise.example.net', mode: 'CONFIG_KEY', urls: null, key: null, tags: [] } });
  });

  it('CR-14.2: a mode change sends the new mode\'s secret and null for the old one', () => {
    const check = edited(CONFIG_KEY_CHANNEL, (draft) => {
      draft.apprise.mode = 'STATELESS_URLS';
      draft.apprise.urls = `tgram://${FAKE_SECRET}`;
    });
    expect(settingsOf(check)).toMatchObject({ apprise: { mode: 'STATELESS_URLS', urls: [`tgram://${FAKE_SECRET}`], key: null } });
  });

  it('CR-14.3: headers untouched are null, none left are [], a list replaces them all', () => {
    expect(settingsOf(edited(webhook))).toMatchObject({ webhook: { headers: null } });
    const replace = (headers: ChannelDraft['webhook']['headers']) =>
      edited(webhook, (draft) => {
        draft.webhook.replaceHeaders = true;
        draft.webhook.headers = headers;
      });
    expect(settingsOf(replace([]))).toMatchObject({ webhook: { headers: [] } });
    const list = [{ name: 'X-One', value: FAKE_SECRET }, { name: 'X-Two', value: FAKE_SECRET }];
    expect(settingsOf(replace(list))).toMatchObject({ webhook: { headers: list } });
  });

  it('F-8: Remove does nothing for an email channel that has no stored password: it stays null', () => {
    const channel = { ...email, settings: { ...email.settings, passwordStored: false } } as NotificationChannel;
    const check = edited(channel, (draft) => {
      draft.email.password.clear = true;
    });
    expect(settingsOf(check)).toMatchObject({ email: { password: null } });
  });
});
