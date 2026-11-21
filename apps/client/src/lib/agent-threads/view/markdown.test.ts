import { describe, expect, it } from 'vitest';
import { inlineParts, markdownBlocks } from './markdown';

describe('the Markdown of a reply', () => {
  it('THR-019: a text is paragraphs, headings, lists and code blocks, in order', () => {
    const blocks = markdownBlocks('# Plan\n\nFirst line\nsecond line\n\n- one\n- two\n\n1. first\n2. second\n\n```ts\nconst a = 1;\n\nconst b = 2;\n```\nDone.');
    expect(blocks.map((block) => block.kind)).toEqual(['heading', 'paragraph', 'list', 'list', 'code', 'paragraph']);
    expect(blocks[1]).toEqual({ kind: 'paragraph', inline: [{ kind: 'text', text: 'First line\nsecond line' }] });
    expect(blocks[2]).toMatchObject({ ordered: false, items: [[{ kind: 'text', text: 'one' }], [{ kind: 'text', text: 'two' }]] });
    expect(blocks[3]).toMatchObject({ ordered: true });
    expect(blocks[4]).toEqual({ kind: 'code', language: 'ts', text: 'const a = 1;\n\nconst b = 2;' });
  });

  it('THR-019: code in a line and bold are parts; a tag stays text', () => {
    expect(inlineParts('Run `bun test` and **wait** <b>here</b>')).toEqual([
      { kind: 'text', text: 'Run ' },
      { kind: 'code', text: 'bun test' },
      { kind: 'text', text: ' and ' },
      { kind: 'bold', text: 'wait' },
      { kind: 'text', text: ' <b>here</b>' },
    ]);
  });

  it('THR-019: a code block with no end takes the rest, and a heading mark inside code is code', () => {
    expect(markdownBlocks('```\n# not a heading\n- not a list')).toEqual([{ kind: 'code', language: '', text: '# not a heading\n- not a list' }]);
  });

  it('THR-019: a longer fence holds a shorter one, and an empty text has no block', () => {
    expect(markdownBlocks('````md\n```\ninner\n```\n````')).toEqual([{ kind: 'code', language: 'md', text: '```\ninner\n```' }]);
    expect(markdownBlocks('  \n')).toEqual([]);
  });
});
