#!/usr/bin/env bash
# The rustc wrapper of .cargo/config.toml: runs the compiler through sccache when sccache is
# on PATH, and directly when it is not, so a machine without sccache builds as before.
#
# It must be bash, not sh: cargo gives the compiler variables such as
# CARGO_BIN_EXE_anima-local-hub, and dash (the sh of Ubuntu) drops each variable whose name is
# not a shell name. The first run of the gates on GitHub failed for that reason.
if command -v sccache >/dev/null 2>&1; then
  exec sccache "$@"
fi
exec "$@"
