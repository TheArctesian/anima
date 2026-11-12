# Vendored Apache Guacamole 1.5.5 `guacamole-common-js` (R-045, R-046)

`guacamole.js` is the Apache Guacamole 1.5.5 JavaScript client, copied without change. Do not edit it.

| | |
|---|---|
| Release | Apache Guacamole 1.5.5, source release `guacamole-client-1.5.5.tar.gz` |
| URL | <https://archive.apache.org/dist/guacamole/1.5.5/source/guacamole-client-1.5.5.tar.gz> (`.sha256` and `.asc` beside it) |
| Release SHA-256 | `8251a4dcec11f0b878f836f8223e81aa049a937f70b06410d320e60acb727258` (matches Apache's `.sha256`) |
| Release signature | `guacamole-client-1.5.5.tar.gz.asc`: GOODSIG and VALIDSIG, made 2024-03-30 by key `F467 E54A CC52 F1D2 7788 2686 5B29 77AE E5E4 518F` (Michael Jumper, CODE SIGNING KEY, `mjumper@apache.org`), the only key in <https://downloads.apache.org/guacamole/KEYS>; checked 2026-11-04 in a throwaway keyring. The key is not certified by a key we trust: the trust anchor is the KEYS file served by apache.org |
| Taken from | `guacamole-client-1.5.5/guacamole-common-js/src/main/webapp/modules/*.js` (34 files) |
| Joined modules SHA-256 | `d5c167c0c3c40aae5fa25177dc56465b947ec24ab1a5a7f60e7009684525c9df` (the file without its last line) |
| Licence | Apache License 2.0; each module keeps its licence header |

1.5.5 replaces 1.5.0 (R-046) because 1.5.0 counts element lengths in UTF-16 units while guacd and the hub count
code points: a non-BMP character in a relayed instruction shifted 1.5.0's parse, so the rest of the element could
be read as forged instructions. The 1.5.5 source still sets `Guacamole.API_VERSION = "1.5.4"`.

## How the file is built

The 34 module files are joined in byte order of their names (`LC_ALL=C`), and one line is appended so that the
client can import it as an ES module:

```sh
cd guacamole-client-1.5.5/guacamole-common-js/src/main/webapp/modules
LC_ALL=C cat $(ls *.js | LC_ALL=C sort) > guacamole.js
echo 'export default Guacamole;' >> guacamole.js
```

`vendor.test.ts` pins the SHA-256 of the file without its last line, so any edit fails the tests.

## Rules

- Only `$lib/sessions/tunnel/` imports this folder (ESLint `no-restricted-imports` and `no-restricted-syntax`).
- `guacamole.d.ts` is hand-written and covers only what the client calls; keep it true to 1.5.5.
- The quality checks skip `guacamole.js` (`quality/generated.txt`); ESLint ignores it.
- `guacamole.js` is vendored whole, so it still holds `Guacamole.SessionRecording`; the client never calls it, because session recording is deferred (R-113, RA-041).
