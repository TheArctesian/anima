// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { settle } from './link/fake/link.fixture';
import { HOME, bar, button, click, entry, file, listed, off, pick, removePanels, resetPanels, status } from './panel.fixture';

beforeEach(resetPanels);
afterEach(removePanels);

describe('RA-032, MOT-009: download', () => {
  it('MOT-009: progress is received over size, exact and linear, with no transition on the value', async () => {
    const { target, socket } = await listed([entry('big.bin', 'file', 200)]);
    await click(button(target, 'Download big.bin'));
    expect(socket.messages().at(-1)).toEqual({ type: 'download', path: `${HOME}/big.bin` });
    socket.text({ type: 'download', size: 200 });
    flushSync();
    expect(bar(target)?.getAttribute('aria-valuenow')).toBe('0');
    const seen: string[] = [];
    for (const length of [50, 50, 100]) {
      socket.bytes(length);
      flushSync();
      seen.push(bar(target)?.getAttribute('aria-valuenow') ?? '');
    }
    expect(seen).toEqual(['0.25', '0.5', '1']);
    expect(bar(target)?.getAttribute('style')).toContain('100%');
    expect(bar(target)?.getAttribute('aria-valuemin')).toBe('0');
    expect(bar(target)?.getAttribute('aria-valuemax')).toBe('1');
  });

  it('RA-032: a finished download is saved under a name reduced to its last clean segment, with the bytes received', async () => {
    const { target, socket, saved } = await listed([entry('rep\u0007ort.txt', 'file', 4)]);
    await click(button(target, 'Download rep\u0007ort.txt'));
    socket.text({ type: 'download', size: 4 });
    socket.bytes(4, 9);
    socket.text({ type: 'done' });
    await settle();
    flushSync();
    expect(saved.map((each) => each.name)).toEqual(['report.txt']);
    expect([...new Uint8Array(await saved[0]?.blob.arrayBuffer())]).toEqual([9, 9, 9, 9]);
    expect(status(target)).toBe('Download finished.');
    expect(bar(target)?.getAttribute('aria-valuenow')).toBe('0');
  });

  it('RA-032: Cancel stops the download, saves nothing and says so', async () => {
    const { target, socket, saved } = await listed([entry('f.bin', 'file', 10)]);
    await click(button(target, 'Download f.bin'));
    socket.text({ type: 'download', size: 10 });
    socket.bytes(3);
    flushSync();
    await click(button(target, 'Cancel'));
    expect(socket.types().at(-1)).toBe('cancel');
    socket.text({ type: 'done' });
    await settle();
    flushSync();
    expect(saved).toEqual([]);
    expect(status(target)).toBe('The transfer was cancelled.');
  });

  it.each([
    ['DOWNLOAD_OFF', 'Downloads are off for this device.'],
    ['NOT_FOUND', 'The device has no file or folder at that place.'],
    ['DENIED', 'The device did not allow that.'],
  ] as const)('RA-032: a download refused with %s shows its fixed text', async (reason, text) => {
    const { target, socket, saved } = await listed([entry('f', 'file')]);
    await click(button(target, 'Download f'));
    socket.text({ type: 'refused', reason });
    await settle();
    flushSync();
    expect(status(target)).toBe(text);
    expect(saved).toEqual([]);
  });
});

describe('RA-032: upload', () => {
  function drop(target: HTMLElement, chosen: File) {
    const event = new Event('drop', { bubbles: true, cancelable: true });
    Object.defineProperty(event, 'dataTransfer', { value: { files: [chosen] } });
    target.querySelector('.files')?.dispatchEvent(event);
  }

  it('RA-032: a picked file is sent to the current directory under its own name, then the folder is listed again', async () => {
    const { target, socket } = await listed([]);
    pick(target, file('notes.txt', 5));
    await settle();
    flushSync();
    expect(socket.messages().at(-1)).toEqual({ type: 'upload', path: `${HOME}/notes.txt`, size: 5 });
    expect(socket.binarySizes()).toEqual([5]);
    socket.text({ type: 'done' });
    await settle();
    flushSync();
    expect(status(target)).toBe('Upload finished.');
    expect(socket.messages().at(-1)).toEqual({ type: 'list', path: HOME });
  });

  it('RA-032: a dropped file is uploaded the same way', async () => {
    const { target, socket } = await listed([]);
    drop(target, file('dropped.bin', 3));
    await settle();
    expect(socket.messages().at(-1)).toEqual({ type: 'upload', path: `${HOME}/dropped.bin`, size: 3 });
  });

  it('RA-032: a local name that is not one plain name is refused before anything is sent', async () => {
    const { target, socket } = await listed([]);
    const before = socket.frames.length;
    for (const name of ['..', 'a/b', '.']) {
      pick(target, file(name));
      await settle();
      flushSync();
    }
    expect(socket.frames).toHaveLength(before);
    expect(status(target)).toBe('That name cannot be used as a file name on the device.');
  });

});

describe('RA-032, MOT-009: upload progress and refusals', () => {
  it('MOT-009: upload progress is the bytes the socket took over the file size', async () => {
    const { target, socket } = await listed([]);
    socket.bufferedAmount = 0;
    pick(target, file('p.bin', 100));
    await settle();
    flushSync();
    expect(bar(target)?.getAttribute('aria-valuenow')).toBe('1');
  });

  it.each([
    ['UPLOAD_OFF', 'Uploads are off for this device.'],
    ['EXISTS', 'A file with this name is already there; Anima never replaces a file.'],
    ['DENIED', 'The device did not allow that.'],
    ['NOT_FOUND', 'The device has no file or folder at that place.'],
  ] as const)('RA-032: an upload refused with %s shows its fixed text and the folder is listed again', async (reason, text) => {
    const { target, socket } = await listed([]);
    pick(target, file('x.txt'));
    await settle();
    socket.text({ type: 'refused', reason });
    await settle();
    flushSync();
    expect(status(target)).toBe(text);
    expect(socket.types().slice(-2)).toEqual(['cancel', 'list']);
  });
});

describe('D-5: one transfer at a time', () => {
  it('D-5: while a download runs every other action is disabled and only Cancel is on', async () => {
    const { target, socket } = await listed([entry('docs', 'dir'), entry('a', 'file'), entry('b', 'file')]);
    await click(button(target, 'Download a'));
    const sent = socket.frames.length;
    expect(off(button(target, 'Download b'))).toBe(true);
    expect(off(button(target, 'docs'))).toBe(true);
    expect(off(button(target, 'Up one folder'))).toBe(true);
    expect(off(button(target, 'Refresh the list'))).toBe(true);
    expect(off(button(target, 'Upload a file'))).toBe(true);
    expect(off(button(target, 'Cancel'))).toBe(false);
    expect(target.querySelectorAll('button:disabled')).toHaveLength(0);
    await click(button(target, 'Download b'));
    expect(socket.frames).toHaveLength(sent);
    socket.text({ type: 'refused', reason: 'DENIED' });
    await settle();
    flushSync();
    expect(off(button(target, 'Download b'))).toBe(false);
    expect(off(button(target, 'Cancel'))).toBe(true);
  });
});

describe('MAT-008, MOT-009: the panel is solid and its bar is not eased', () => {
  const source = readFileSync(`${import.meta.dirname}/FilesPanel.svelte`, 'utf8');
  const style = source.slice(source.indexOf('<style>')).replaceAll(/\/\*[\s\S]*?\*\//g, '');

  it('MAT-008: no blur, translucency, filter or overlay position in the panel stylesheet', () => {
    for (const forbidden of ['backdrop-filter', 'filter:', 'opacity', 'position: absolute', 'position: fixed', 'transparent', 'color-mix']) {
      expect(style, forbidden).not.toContain(forbidden);
    }
    expect(style).toContain('background: var(--surface-raised)');
  });

  it('MOT-009: nothing in the panel stylesheet transitions or animates', () => {
    expect(style).not.toMatch(/transition|animation|@keyframes/);
  });

  it('UI-013: the folder-name button is at least the minimum target tall and the status reserves two lines', () => {
    const rule = (selector: string) => new RegExp(`${selector.replace('.', '\\.')} \\{[^}]*\\}`).exec(style)?.[0] ?? '';
    expect(rule('.open')).toContain('min-block-size: var(--target-min-desktop)');
    expect(rule('.status')).toContain('-webkit-line-clamp: 2');
    expect(rule('.entry')).toContain('grid-template-columns: subgrid');
  });
});
