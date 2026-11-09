import { describe, expect, it } from 'vitest';
import { FAKE_SECRET, FAKE_WEBHOOK_URL, fixtureChannel, FIXTURE_SETTINGS } from '../notifications.fixture';
import { forgetSecrets, newDraft, savedDraft, storedOf, type ChannelDraft } from './draft';
import * as refusals from './input';
import { APPRISE_KEY_WRONG, APPRISE_URLS_WRONG, checkDraft, NAME_MISSING, RECIPIENTS_WRONG, TOPIC_WRONG, URL_MISSING, URL_WRONG } from './input';

type DraftCheck = ReturnType<typeof checkDraft>;

const { placeholders, template } = FIXTURE_SETTINGS;

function saved(kind: Parameters<typeof fixtureChannel>[0], edit: (draft: ChannelDraft) => void = () => undefined): DraftCheck {
  const channel = fixtureChannel(kind);
  const draft = savedDraft(channel, template);
  edit(draft);
  return checkDraft(draft, storedOf(channel), placeholders);
}

function created(edit: (draft: ChannelDraft) => void): DraftCheck {
  const draft = newDraft(template);
  draft.name = 'new-channel';
  edit(draft);
  return checkDraft(draft, null, placeholders);
}

function settingsOf(check: DraftCheck): unknown {
  if (!check.ok) throw new Error(`refused: ${check.problems.map((problem) => problem.field).join(', ')}`);
  return check.input.settings;
}

describe('a stored optional secret', () => {
  it('F-8: typed is sent, an untouched secret is null, a cleared one is ""', () => {
    const token = (value: string, clear: boolean) =>
      settingsOf(
        saved('NTFY', (draft) => {
          draft.ntfy.token = { value, clear };
        }),
      );
    expect(token(FAKE_SECRET, false)).toMatchObject({ ntfy: { token: FAKE_SECRET } });
    expect(token(FAKE_SECRET, true)).toMatchObject({ ntfy: { token: FAKE_SECRET } });
    expect(token('', false)).toMatchObject({ ntfy: { token: null } });
    expect(token('', true)).toMatchObject({ ntfy: { token: '' } });
  });
});

describe('an edited channel\'s secrets', () => {
  it('F-8: no secret field is filled from the saved channel', () => {
    for (const kind of ['WEBHOOK', 'NTFY', 'EMAIL', 'APPRISE'] as const) {
      const draft = savedDraft(fixtureChannel(kind), template);
      expect([draft.webhook.url, draft.ntfy.topic, draft.ntfy.token.value, draft.email.password.value, draft.apprise.urls, draft.apprise.key]).toEqual(['', '', '', '', '', '']);
      expect(draft.webhook.headers).toEqual([]);
    }
  });

  it('F-8, NOT-002: every untouched secret is sent as null, so the hub keeps it', () => {
    expect(settingsOf(saved('WEBHOOK'))).toEqual({ webhook: { url: null, method: 'POST', headers: null } });
    expect(settingsOf(saved('NTFY'))).toEqual({ ntfy: { server: 'https://ntfy.example.net', topic: null, token: null, priority: 3 } });
    expect(settingsOf(saved('EMAIL'))).toEqual({
      email: { host: 'mail.example.net', port: 587, security: 'STARTTLS', username: 'anima', password: null, from: 'anima@example.net', to: ['ops@example.net'] },
    });
    expect(settingsOf(saved('APPRISE'))).toEqual({ apprise: { server: 'https://apprise.example.net', mode: 'STATELESS_URLS', urls: null, key: null, tags: ['ops'] } });
  });

  it('F-8: a cleared optional secret is sent as "" (ntfy token, SMTP password)', () => {
    const ntfy = saved('NTFY', (draft) => {
      draft.ntfy.token.clear = true;
    });
    expect(settingsOf(ntfy)).toMatchObject({ ntfy: { token: '', topic: null } });
    const email = saved('EMAIL', (draft) => {
      draft.email.password.clear = true;
    });
    expect(settingsOf(email)).toMatchObject({ email: { password: '' } });
  });

  it('F-8: "Remove" does nothing for a secret the hub does not hold: it stays null', () => {
    const channel = { ...fixtureChannel('NTFY'), settings: { __typename: 'NtfySettings' as const, server: 'https://ntfy.sh', priority: 3, tokenStored: false } };
    const draft = savedDraft(channel, template);
    draft.ntfy.token.clear = true;
    expect(settingsOf(checkDraft(draft, storedOf(channel), placeholders))).toMatchObject({ ntfy: { token: null } });
  });

  it('F-8: a typed secret replaces the stored one', () => {
    const check = saved('WEBHOOK', (draft) => {
      draft.webhook.url = ` ${FAKE_WEBHOOK_URL} `;
    });
    expect(settingsOf(check)).toMatchObject({ webhook: { url: FAKE_WEBHOOK_URL } });
  });

});

describe('an edited channel\'s headers, mode and template', () => {
  it('F-8: replacing the headers needs every value again; none left removes them all', () => {
    const replaced = saved('WEBHOOK', (draft) => {
      draft.webhook.replaceHeaders = true;
      draft.webhook.headers = [{ name: 'X-Anima-Source', value: FAKE_SECRET }];
    });
    expect(settingsOf(replaced)).toMatchObject({ webhook: { headers: [{ name: 'X-Anima-Source', value: FAKE_SECRET }] } });
    const emptied = saved('WEBHOOK', (draft) => {
      draft.webhook.replaceHeaders = true;
    });
    expect(settingsOf(emptied)).toMatchObject({ webhook: { headers: [] } });
    const missing = saved('WEBHOOK', (draft) => {
      draft.webhook.replaceHeaders = true;
      draft.webhook.headers = [{ name: 'X-Anima-Source', value: '' }];
    });
    expect(missing.ok ? [] : missing.problems.map((problem) => problem.field)).toEqual(['webhook.headers']);
  });

  it('F-8: an Apprise channel that changes its mode needs the new mode\'s key; the same mode keeps its URLs', () => {
    const switched = saved('APPRISE', (draft) => {
      draft.apprise.mode = 'CONFIG_KEY';
    });
    expect(switched).toEqual({ ok: false, problems: [{ field: 'apprise.key', message: APPRISE_KEY_WRONG }] });
    const keyed = saved('APPRISE', (draft) => {
      draft.apprise.mode = 'CONFIG_KEY';
      draft.apprise.key = 'FAKE_SECRET_key';
    });
    expect(settingsOf(keyed)).toMatchObject({ apprise: { mode: 'CONFIG_KEY', key: 'FAKE_SECRET_key', urls: null } });
  });

  it('NOT-007: turning off the own template sends null, the default template', () => {
    const email = fixtureChannel('EMAIL');
    expect(email.template).not.toBeNull();
    const own = saved('EMAIL');
    expect(own.ok && own.input.template).toEqual(email.template);
    const reset = saved('EMAIL', (draft) => {
      draft.ownTemplate = false;
    });
    expect(reset.ok && reset.input.template).toBeNull();
  });
});

describe('a new channel', () => {
  it('NOT-002: the required secret of each kind must be typed', () => {
    const fieldsOf = (check: DraftCheck) => (check.ok ? [] : check.problems.map((problem) => problem.message));
    expect(fieldsOf(created(() => undefined))).toEqual([URL_MISSING]);
    expect(fieldsOf(created((draft) => { draft.kind = 'NTFY'; }))).toEqual([TOPIC_WRONG]);
    expect(fieldsOf(created((draft) => { draft.kind = 'APPRISE'; draft.apprise.server = 'https://apprise.example.net'; }))).toEqual([APPRISE_URLS_WRONG]);
    expect(fieldsOf(created((draft) => { draft.name = ' '; draft.kind = 'EMAIL'; draft.email.host = 'smtp.example.net'; draft.email.from = 'a@example.net'; }))).toEqual([NAME_MISSING, RECIPIENTS_WRONG]);
  });

  it('NOT-002, NOT-003: a webhook with headers, on by default, goes out as one oneOf settings input', () => {
    const check = created((draft) => {
      draft.onByDefault = true;
      draft.webhook = { url: FAKE_WEBHOOK_URL, method: 'PUT', replaceHeaders: true, headers: [{ name: 'Authorization', value: FAKE_SECRET }, { name: '', value: '' }] };
    });
    expect(check).toEqual({
      ok: true,
      input: {
        name: 'new-channel',
        onByDefault: true,
        settings: { webhook: { url: FAKE_WEBHOOK_URL, method: 'PUT', headers: [{ name: 'Authorization', value: FAKE_SECRET }] } },
        template: null,
      },
    });
  });

  it('NOT-002: an optional secret left empty on a new channel is null, never ""', () => {
    const check = created((draft) => {
      draft.kind = 'NTFY';
      draft.ntfy.topic = 'FAKE_SECRET_topic';
    });
    expect(settingsOf(check)).toEqual({ ntfy: { server: 'https://ntfy.sh', topic: 'FAKE_SECRET_topic', token: null, priority: 3 } });
  });

  it('F-8, LOG-007: no refusal repeats what was typed into a secret field', () => {
    const check = created((draft) => {
      draft.webhook.url = `ftp://${FAKE_SECRET}`;
      draft.webhook.headers = [{ name: `${FAKE_SECRET} x`, value: FAKE_SECRET }];
    });
    expect(check.ok).toBe(false);
    expect(JSON.stringify(check)).not.toContain(FAKE_SECRET);
    expect(check.ok ? [] : check.problems[0]?.message).toBe(URL_WRONG);
  });

  it('D-21: a template with a placeholder outside the list is refused before sending', () => {
    const check = created((draft) => {
      draft.webhook.url = FAKE_WEBHOOK_URL;
      draft.ownTemplate = true;
      draft.template = { title: '{device} {hostname}', body: '{check}' };
    });
    expect(check.ok ? [] : check.problems.map((problem) => problem.field)).toEqual(['template.title']);
  });
});

describe('forgetSecrets', () => {
  it('F-8: empties every secret and keeps the rest', () => {
    const draft = newDraft(template);
    draft.name = 'kept';
    draft.webhook.url = FAKE_SECRET;
    draft.webhook.headers = [{ name: 'X-Kept', value: FAKE_SECRET }];
    draft.ntfy.topic = FAKE_SECRET;
    draft.ntfy.token.value = FAKE_SECRET;
    draft.email.password.value = FAKE_SECRET;
    draft.apprise.urls = FAKE_SECRET;
    draft.apprise.key = FAKE_SECRET;
    forgetSecrets(draft);
    expect(JSON.stringify(draft)).not.toContain(FAKE_SECRET);
    expect([draft.name, draft.webhook.headers[0]?.name]).toEqual(['kept', 'X-Kept']);
  });
});

describe('refusal sentences', () => {
  it('MOT-007: each fits the one line kept under a field of the editor at 390 px (36 characters)', () => {
    const sentences = Object.values(refusals).filter((value) => typeof value === 'string');
    expect(sentences.length).toBeGreaterThan(10);
    expect(sentences.filter((sentence) => sentence.length > 36)).toEqual([]);
  });
});
