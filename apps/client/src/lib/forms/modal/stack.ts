/** The dialogs that are open, oldest first. Only the last one answers keys and focus outside every dialog. */
const openDialogs: object[] = [];

export interface OpenDialog {
  isTopmost(): boolean;
  /** Takes the dialog off the stack; the one below it is topmost again. */
  close(): void;
}

/** Puts a dialog on top of the stack. */
export function openDialog(): OpenDialog {
  const entry = {};
  openDialogs.push(entry);
  return {
    isTopmost: () => openDialogs.at(-1) === entry,
    close: () => {
      const at = openDialogs.indexOf(entry);
      if (at !== -1) openDialogs.splice(at, 1);
    },
  };
}
