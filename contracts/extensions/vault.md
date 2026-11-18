# K-E5 — Vault: data classes, data keys and operational keys (engine)

Status: approved for the engine build (R-148 of 2026-11-18). Section 1 is written with engine task E0-5, because the file store and the location samples seal with a data key. Sections 2 and later (operational keys, client-sealed records, reveal) are written by E0-6 T1. Change it only through a programme ruling (CON-001). Requirement IDs: VLT-001 to VLT-007 of `docs/plans/extensions-plan.md`; R-146, R-147.

## 1. Data keys (VLT-003, R-146; `crates/hub/src/engine/data_keys.rs`, leaf)

A data key seals the records and the files of one extension on the hub. It is not the credential key (`ANIMA_KEY_FILE`, SEC-004), and no code seals a record or a file with the credential key.

- `ANIMA_DATA_KEY_DIR` names a folder. Each data key is one file in it, `<name>.key`: 32 random bytes in base64, in a file that only the hub user can read (mode 0600). `name` is the key of an extension (K-E7) or an engine name (`engine`, `locations`), in the form `^[a-z][a-z0-9_]{0,39}$`.
- When a domain first asks for a key that has no file, the hub makes the file. Thus a new extension needs no step of the operator. A file that exists is never written again.
- With no `ANIMA_DATA_KEY_DIR`, a domain that seals answers `UNAVAILABLE` and the other domains work. A local hub and a hub of the pool put the folder `data-keys/` in their root, beside `vault.key`.
- At start the hub compares the identifier of each data key with the identifier of the credential key and with each other. Two equal keys stop the start (VLT-003).
- The form of a sealed value is the form of the credential vault (XChaCha20-Poly1305, a context text in the additional data, the identifier of the key before the nonce). A value opens only with its key and its context. The context of a record is `<schema>.<table>:<id>` or the text that its contract gives. A file uses the stream form of K-E4 Section 9.
- A data key has no version now. To change a data key is later work with the operational keys (Section 2, E0-6).

```rust
pub struct DataKeys { /* the folder */ }
pub struct DataKey { /* 32 bytes, zeroed on drop; no Debug of the bytes */ }
impl DataKeys {
    pub fn open(folder: Option<&Path>, credential_key_id: &[u8]) -> Result<Self, DataKeyError>;
    pub fn key(&self, name: &str) -> Result<DataKey, DataKeyError>;      // Unavailable with no folder
}
impl DataKey {
    pub fn name(&self) -> &str;
    pub fn seal(&self, context: &str, plain: &[u8]) -> Vec<u8>;
    pub fn open(&self, context: &str, sealed: &[u8]) -> Result<Vec<u8>, DataKeyError>;   // WrongKey, Damaged
}
```

- No log line, error text, event or audit entry holds a key or a plain value (LOG-007).
- The hub can open Class B and Class C data (VLT-007, R-146). It has no key for Class A (VLT-001): Section 3, with E0-6.
