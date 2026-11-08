/**
 * "Reduce motion" in Settings (MOT-043, D-28): System follows the OS, On reduces and Off keeps full motion whatever the
 * OS says. The ids are the `data-motion` values the generated `tokens.css` and `$lib/motion/preference` read (F-7).
 */
export const MOTION_CHOICES = [
  { id: 'system', label: 'System' },
  { id: 'reduce', label: 'On' },
  { id: 'full', label: 'Off' },
] as const;

export type MotionChoice = (typeof MOTION_CHOICES)[number]['id'];

export const DEFAULT_MOTION: MotionChoice = 'system';

const STORAGE_KEY = 'anima.motion';

function isMotionChoice(text: string | null): text is MotionChoice {
  return MOTION_CHOICES.some((choice) => choice.id === text);
}

/** The choice the operator made on this device; System when nothing valid is stored or the storage refuses to be read. */
export function storedMotion(storage: Storage): MotionChoice {
  let text: string | null;
  try {
    text = storage.getItem(STORAGE_KEY);
  } catch {
    return DEFAULT_MOTION;
  }
  return isMotionChoice(text) ? text : DEFAULT_MOTION;
}

/** Save the choice on this device; throws when the storage refuses (quota, privacy modes). */
export function storeMotion(storage: Storage, choice: MotionChoice): void {
  storage.setItem(STORAGE_KEY, choice);
}

/**
 * Mark the document with the choice: `data-motion="reduce"` or `"full"`, and no attribute for System, so CSS tokens and
 * JS springs follow the OS as they did before the setting existed (MOT-040, MOT-041).
 */
export function applyMotion(root: HTMLElement, choice: MotionChoice): void {
  if (choice === 'system') delete root.dataset['motion'];
  else root.dataset['motion'] = choice;
}
