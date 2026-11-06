// The version a Tauri updater signature was made for (UPD-002, UPD-006). The apps set
// `requireSignedVersion`, so a `.sig` with no version, or another one, makes every installed app
// refuse the update without a message. The format is the one tauri-plugin-updater reads: the
// `.sig` file is base64 of the minisign text, and the trusted comment is tab-separated
// `key:value` pairs, for example `timestamp:1700000000\tfile:Anima.app.tar.gz\tversion:0.1.7`.

export class SignedVersionError extends Error {
  constructor(file: string, expected: string, found: string | undefined) {
    super(`${file}: signed for version ${found ?? "(none)"}, expected ${expected}`);
    this.name = "SignedVersionError";
  }
}

/** The `version:` field of the trusted comment, or undefined when the signature has none. */
export function signedVersion(signatureFile: string): string | undefined {
  const text = Buffer.from(signatureFile.trim(), "base64").toString("utf8");
  const prefix = "trusted comment: ";
  const comment = text.split("\n").find((line) => line.startsWith(prefix));
  return comment
    ?.slice(prefix.length)
    .split("\t")
    .find((field) => field.startsWith("version:"))
    ?.slice("version:".length);
}

/** Throws unless the signature was made for exactly this version. */
export function assertSignedVersion(file: string, signatureFile: string, version: string): void {
  const found = signedVersion(signatureFile);
  if (found !== version) throw new SignedVersionError(file, version, found);
}
