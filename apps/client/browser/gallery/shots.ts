import type { Locator, Page } from '@playwright/test';
import type { Fixtures } from '../synthetic/fixtures';

// What the gallery shoots: the passes (a theme at a window width, plus one OpenDyslexic pass, UI-019) and the screens.

export type ThemeName = 'dark' | 'light';

export interface Mode {
  readonly theme: ThemeName;
  readonly width: number;
  readonly dyslexic: boolean;
  /** The folder under `gallery-shots/`, for example `dark-1280` or `dark-1280-dyslexic`. */
  readonly folder: string;
}

export const WIDTHS = [800, 1280, 1600] as const;
export const HEIGHT = 900;
const THEMES: readonly ThemeName[] = ['dark', 'light'];
const DYSLEXIC_WIDTH = 1280;

function mode(theme: ThemeName, width: number, dyslexic: boolean): Mode {
  return { theme, width, dyslexic, folder: `${theme}-${String(width)}${dyslexic ? '-dyslexic' : ''}` };
}

/** Dark first, as the app's default. */
export const MODES: readonly Mode[] = [
  ...THEMES.flatMap((theme) => WIDTHS.map((width) => mode(theme, width, false))),
  mode('dark', DYSLEXIC_WIDTH, true),
];

/** The settings the app reads from `localStorage` before its first render: theme (`$lib/theme`) and typeface (`$lib/typeface`). */
export function storedSettings(chosen: Mode): Record<string, string> {
  const typeface = chosen.dyslexic ? 'opendyslexic' : 'fira-code';
  return { 'anima.theme': chosen.theme, 'anima.typeface.ui': typeface, 'anima.typeface.mono': typeface };
}

/** One state of the app to photograph. The runner numbers the screens in list order and shoots each in every mode. */
export interface Screen {
  /** The file name after the number, for example `devices-grid`. */
  readonly name: string;
  /** Sign-in screens start signed out. */
  readonly signedOut?: true;
  /** Reshapes the hub's fixtures, for the empty states. */
  readonly fixtures?: (loaded: Fixtures) => Fixtures;
  /** Gets the page into the state: navigates, clicks, and waits for what the state shows. */
  readonly open: (page: Page) => Promise<void>;
  /** Shoots this element instead of the whole page, for a section or a dialog taller than the window. */
  readonly target?: (page: Page) => Locator;
  /** The hub holds an answer on purpose, so the page never goes network-quiet: shoot without waiting for it. */
  readonly loading?: true;
  /** Shoots the window, not the whole scrolling page, for overlays and scrolled positions. */
  readonly windowOnly?: true;
}
