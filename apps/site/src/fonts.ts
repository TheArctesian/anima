// The font faces the page uses, taken from the client's font rules so that the site ships the same files (SITE-008, SITE-009).
// Pure: the text of `apps/client/src/styles/fonts.css` comes in as a parameter.

/** The font files of the page, in `apps/client/src/styles/fonts/` (UI-010, UI-011). */
export const FONT_FILES = ["IBMPlexSans-Regular.woff2", "IBMPlexSans-Bold.woff2", "FiraCode-Regular.woff2"] as const;

/** The font file the client does not ship in the form the site expects, or a rule that names none of it. */
export class FontRuleError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "FontRuleError";
  }
}

/** The `@font-face` rules of `fontsCss` that load one of `FONT_FILES`, one for each file. */
export function siteFontRules(fontsCss: string): string {
  const rules = fontsCss.match(/@font-face\s*\{[^}]*\}/g) ?? [];
  const chosen = FONT_FILES.map((file) => {
    const rule = rules.find((candidate) => candidate.includes(`./fonts/${file}`));
    if (rule === undefined) throw new FontRuleError(`The client font rules have no @font-face for ${file}.`);
    return rule;
  });
  return `/* Taken from apps/client/src/styles/fonts.css at build time. */\n${chosen.join("\n\n")}\n`;
}
