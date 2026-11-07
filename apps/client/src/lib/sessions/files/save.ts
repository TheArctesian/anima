/** Hands `blob` to the operator as a file named `name`. */
export type BlobSaver = (name: string, blob: Blob) => void;

/** The browser's save: an object URL on a hidden `<a download>`, clicked, then revoked. */
export const saveBlob: BlobSaver = (name, blob) => {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = name;
  anchor.hidden = true;
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
};
