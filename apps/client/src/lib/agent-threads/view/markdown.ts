/**
 * The Markdown of a reply or a prompt as parts that the page draws (THR-019). The text of a session is not trusted: this
 * gives parts of plain text, and the page builds its own elements from them. No part is ever markup of the session, so a
 * tag in a reply shows as text.
 */

export interface Inline {
  readonly kind: 'text' | 'code' | 'bold';
  readonly text: string;
}

export type Block =
  | { readonly kind: 'paragraph'; readonly inline: readonly Inline[] }
  | { readonly kind: 'heading'; readonly level: number; readonly inline: readonly Inline[] }
  | { readonly kind: 'list'; readonly ordered: boolean; readonly items: readonly (readonly Inline[])[] }
  | { readonly kind: 'code'; readonly language: string; readonly text: string };

const FENCE = /^\s{0,3}(`{3,}|~{3,})\s*([\w+#.-]*)\s*$/;
const HEADING = /^\s{0,3}(#{1,6})\s+(.*)$/;
const ITEM = /^\s*(?:([-*+])|(\d{1,9})[.)])\s+(.*)$/;
const INLINE = /(`+)([^`]|[^`].*?[^`])\1(?!`)|\*\*([^*\n]+)\*\*/g;

/** A line of text as parts: code between backticks, bold between two stars, and text. */
export function inlineParts(line: string): readonly Inline[] {
  const parts: Inline[] = [];
  let from = 0;
  for (const match of line.matchAll(INLINE)) {
    if (match.index > from) parts.push({ kind: 'text', text: line.slice(from, match.index) });
    parts.push(match[2] !== undefined ? { kind: 'code', text: match[2] } : { kind: 'bold', text: match[3] ?? '' });
    from = match.index + match[0].length;
  }
  if (from < line.length) parts.push({ kind: 'text', text: line.slice(from) });
  return parts;
}

/** The lines of a fenced code block that opens at `lines[at]`, and the place after its end; a block with no end takes the rest. */
function fenced(lines: readonly string[], at: number, fence: string): { readonly text: string; readonly next: number } {
  const body: string[] = [];
  let index = at + 1;
  for (; index < lines.length; index += 1) {
    const line = lines[index] ?? '';
    const close = FENCE.exec(line);
    if (close?.[1] !== undefined && close[1].startsWith(fence) && close[2] === '') return { text: body.join('\n'), next: index + 1 };
    body.push(line);
  }
  return { text: body.join('\n'), next: index };
}

/** The blocks of `text`: code blocks, headings, lists and paragraphs. A line break in a paragraph stays a line break. */
export function markdownBlocks(text: string): readonly Block[] {
  const lines = text.replace(/\r\n?/g, '\n').split('\n');
  const blocks: Block[] = [];
  let paragraph: string[] = [];
  let list: { ordered: boolean; items: (readonly Inline[])[] } | null = null;
  const close = (): void => {
    if (paragraph.length > 0) blocks.push({ kind: 'paragraph', inline: inlineParts(paragraph.join('\n')) });
    if (list !== null) blocks.push({ kind: 'list', ordered: list.ordered, items: list.items });
    paragraph = [];
    list = null;
  };
  /** A line that opens no code block: a heading, an item of a list, a blank line, or a line of a paragraph. */
  const take = (line: string): void => {
    const heading = HEADING.exec(line);
    const item = ITEM.exec(line);
    if (heading !== null) {
      close();
      blocks.push({ kind: 'heading', level: heading[1]?.length ?? 1, inline: inlineParts(heading[2] ?? '') });
    } else if (item !== null) {
      const ordered = item[2] !== undefined;
      if (list?.ordered !== ordered) close();
      list ??= { ordered, items: [] };
      list.items.push(inlineParts(item[3] ?? ''));
    } else if (line.trim() === '') {
      close();
    } else {
      if (list !== null) close();
      paragraph.push(line);
    }
  };
  for (let index = 0; index < lines.length; ) {
    const line = lines[index] ?? '';
    const fence = FENCE.exec(line);
    if (fence?.[1] === undefined) {
      take(line);
      index += 1;
      continue;
    }
    close();
    const code = fenced(lines, index, fence[1]);
    blocks.push({ kind: 'code', language: fence[2] ?? '', text: code.text });
    index = code.next;
  }
  close();
  return blocks;
}
