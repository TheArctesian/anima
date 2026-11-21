/**
 * What each extension that is not built will do, in one sentence from its section of the extensions plan
 * (docs/plans/extensions-plan.md, Section 4). The hub sends no description of an extension, so the words are here,
 * by the key of the extension. A sentence gives no date and promises nothing that the plan does not hold (R-223).
 */
const WILL_DO: Readonly<Record<string, string>> = {
  map: 'It will show where you were, the places that you use, and where your sites and belongings are.',
  writing: 'It will keep your notes, drafts and documents in Anima.ing.',
  secrets: 'It will keep global secrets and project secrets on the hub, and give them to a command or to the model gateway.',
  people: 'It will be a directory of the people that you know, each with their accounts and your history of contact.',
  money: 'It will show balances, transactions, investments, recurring costs and net worth across your accounts. It will never move money.',
  history: 'It will show what happened, from all sources, on one timeline.',
  health: 'It will keep health and fitness data from your devices and reports in one place. It will not diagnose.',
  documents: 'It will keep copies of your important documents and their expiry dates.',
  reminders: 'It will be the page of your reminders: the list and the editor of the rules.',
  assistant_sessions: 'It will keep a record of your AI coding sessions, by repository.',
  wardrobe: 'It will keep a record of each item of clothing, with a photo and its place.',
  library: 'It will keep a record of each book, with its place and its reading status, and of the films that you watched.',
  model_gateway: 'It will give each of your computers one address for AI model requests. The keys stay on the hub.',
};

/**
 * What the extension `key`, which is not built, will do, or nothing when the plan says nothing. The row shows the tag
 * "Coming soon" beside it (R-223).
 */
export function willDo(key: string): string {
  return WILL_DO[key] ?? '';
}
