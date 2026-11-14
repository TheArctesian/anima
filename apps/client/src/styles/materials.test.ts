import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const src = fileURLToPath(new URL('../', import.meta.url));
const stylesDir = fileURLToPath(new URL('./', import.meta.url));
const read = (path: string): string => readFileSync(path, 'utf8');
const materials = read(`${stylesDir}materials.css`);

function svelteFiles(folder: string): string[] {
  return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
    const path = `${folder}${entry.name}`;
    if (entry.isDirectory()) return svelteFiles(`${path}/`);
    return entry.name.endsWith('.svelte') ? [path] : [];
  });
}

/** The declarations of the first rule whose selector is exactly `selector`. */
function rule(selector: string): string {
  const css = materials.replaceAll(/\/\*[\s\S]*?\*\//g, '');
  const start = css.search(new RegExp(`(^|\\})\\s*${selector.replace('.', '\\.')} \\{`));
  return start === -1 ? '' : css.slice(start, css.indexOf('}', start + 1));
}

describe('MAT-010: the one place that makes glass', () => {
  for (const name of ['chrome', 'panel', 'overlay']) {
    it(`MAT-010: material-${name} reads only its tokens and sets both backdrop-filter spellings (WebKit)`, () => {
      const declarations = rule(`.material-${name}`);
      expect(declarations).toContain(`background-color: var(--material-${name}-bg)`);
      const filter = `blur(var(--material-${name}-blur)) saturate(var(--material-${name}-saturate))`;
      expect(declarations).toContain(`-webkit-backdrop-filter: ${filter}`);
      expect(declarations).toContain(`backdrop-filter: ${filter}`);
    });
  }

  it('MAT-010: the edge is an inset shadow (no layout shift) and the sheen a gradient, both from tokens', () => {
    const shared = materials.slice(0, materials.indexOf('.material-chrome {'));
    expect(shared).toContain('inset 0 0 0 var(--border-thin) var(--material-edge)');
    expect(shared).toContain('linear-gradient(to bottom, var(--material-sheen), transparent)');
  });

  it('MAT-010: no number, colour or time is written in the stylesheet, only tokens', () => {
    const code = materials.replaceAll(/\/\*[\s\S]*?\*\//g, '');
    expect(code).not.toMatch(/#[0-9a-f]{3,8}\b|\d+(\.\d+)?(px|ms|s|%)|rgba?\(|hsla?\(/i);
  });

  it('MAT-010: no component repeats a blur of its own, and the logs panel is opaque (MAT-009)', () => {
    const repeating = [...svelteFiles(`${src}lib/`), ...svelteFiles(`${src}routes/`)].filter((path) => /backdrop-filter:\s*blur|material-\w+-blur/.test(read(path)));
    expect(repeating.map((path) => path.slice(src.length))).toEqual([]);
    expect(read(`${src}lib/events/panel/LogsPanel.svelte`)).toContain('background: var(--surface-raised);');
  });

  it('MAT-010: the menus, the tip and the modal glass layer take the shared class; the bar is opaque (UR-7, D15)', () => {
    expect(read(`${src}lib/nav/Nav.svelte`)).not.toContain('material-chrome');
    const uses: Record<string, string> = {
      'lib/devices/toolbar/Menu.svelte': 'class="panel material-overlay"',
      'lib/devices/actions/WebMenu.svelte': 'class="panel material-overlay"',
      'lib/devices/screen-sharing/ScreenSharingButton.svelte': 'class="panel material-overlay"',
      'lib/overlay/helptip/HelpTip.svelte': 'class="panel material-overlay"',
      'lib/forms/modal/Modal.svelte': 'class="glass-layer material-overlay"',
    };
    for (const [file, marker] of Object.entries(uses)) expect(read(`${src}${file}`), file).toContain(marker);
  });
});

describe('MAT-002: no material on another material', () => {
  it('MAT-002: an overlay inside a material, or inside the content of a modal, is solid', () => {
    const selector = ':is(.material-chrome, .material-panel, .material-overlay) .material-overlay,\n.frosted-host > :not(.glass-layer) .material-overlay';
    expect(materials).toContain(selector);
    const solid = materials.slice(materials.indexOf(selector));
    expect(solid).toContain('background-color: var(--surface-overlay)');
    expect(solid).toContain('backdrop-filter: none');
  });

  it('MAT-002: the modal surface carries no blur; its glass is a layer behind the content, so a fixed HelpTip keeps the window as its frame', () => {
    const modal = read(`${src}lib/forms/modal/Modal.svelte`);
    const style = modal.slice(modal.indexOf('<style>'));
    expect(style).not.toContain('backdrop-filter');
    expect(modal).toContain('class="surface frosted-host"');
  });
});

describe('MAT-008: no material over a live remote display', () => {
  it('MAT-008: nothing in the session views uses a material class or token', () => {
    for (const path of svelteFiles(`${src}lib/sessions/`)) expect(read(path), path).not.toMatch(/material-(chrome|panel|overlay)/);
  });

  it('MAT-008: the dialogs that can open over a live session are solid', () => {
    for (const file of ['pane/LinkConfirm', 'hostkey/HostKeyModal', 'credentials/CredentialPrompt', 'certificate/CertificateModal']) {
      const source = read(`${src}lib/sessions/view/${file}.svelte`);
      const opening = source.slice(source.indexOf('<Modal'), source.indexOf('<Modal') + 160);
      expect(opening, file).toMatch(/\ssolid\s/);
    }
  });
});
