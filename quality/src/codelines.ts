interface Syntax {
  lineComment: boolean;
  blocks: [open: string, close: string][];
  quotes: string;
  rustChars: boolean;
}

const C_BLOCK: [string, string] = ["/*", "*/"];
const SYNTAX: Record<string, Syntax> = {
  ".rs": { lineComment: true, blocks: [C_BLOCK], quotes: '"', rustChars: true },
  ".ts": { lineComment: true, blocks: [C_BLOCK], quotes: "\"'`", rustChars: false },
  ".js": { lineComment: true, blocks: [C_BLOCK], quotes: "\"'`", rustChars: false },
  ".svelte": {
    lineComment: true,
    blocks: [C_BLOCK, ["<!--", "-->"]],
    quotes: "\"'`",
    rustChars: false,
  },
  ".css": { lineComment: false, blocks: [C_BLOCK], quotes: "\"'", rustChars: false },
};

const RUST_CHAR = /^'(?:\\.[^']*|[^\\'])'/;

interface Step {
  keep: string;
  next: number;
}

function skipBlock(text: string, at: number, open: string, close: string): Step {
  const closeAt = text.indexOf(close, at + open.length);
  const end = closeAt === -1 ? text.length : closeAt + close.length;
  return { keep: text.slice(at, end).replace(/[^\n]/g, ""), next: end };
}

function skipLine(text: string, at: number): Step {
  const newline = text.indexOf("\n", at);
  return { keep: "", next: newline === -1 ? text.length : newline };
}

function readString(text: string, at: number, quote: string): Step {
  let end = at + 1;
  while (end < text.length && text.charAt(end) !== quote) {
    if (text.charAt(end) === "\\") end++;
    else if (text.charAt(end) === "\n" && quote !== "`") break;
    end++;
  }
  const next = text.charAt(end) === quote ? end + 1 : end;
  return { keep: text.slice(at, next), next };
}

function stepAt(text: string, at: number, syntax: Syntax): Step {
  for (const [open, close] of syntax.blocks) {
    if (text.startsWith(open, at)) return skipBlock(text, at, open, close);
  }
  if (syntax.lineComment && text.startsWith("//", at)) return skipLine(text, at);
  const char = text.charAt(at);
  if (syntax.rustChars && char === "'") {
    const literal = RUST_CHAR.exec(text.slice(at, at + 12));
    if (literal) return { keep: literal[0], next: at + literal[0].length };
  }
  if (syntax.quotes.includes(char)) return readString(text, at, char);
  return { keep: char, next: at + 1 };
}

/** Remove comments, keeping newlines. Unknown extensions are returned unchanged. */
export function stripComments(text: string, extension: string): string {
  const syntax = SYNTAX[extension];
  if (!syntax) return text;
  const parts: string[] = [];
  let at = 0;
  while (at < text.length) {
    const step = stepAt(text, at, syntax);
    parts.push(step.keep);
    at = step.next;
  }
  return parts.join("");
}

/** Lines that hold code: not blank, not only comment (CQ-010). */
export function countCodeLines(text: string, extension: string): number {
  return stripComments(text, extension)
    .split("\n")
    .filter((line) => line.trim() !== "").length;
}
