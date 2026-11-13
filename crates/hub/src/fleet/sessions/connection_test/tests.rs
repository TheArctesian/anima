//! Unit tests of the pure parts of reading what a device presents: the X.224 exchange and the
//! VNC banner. The readers are tested through GraphQL in `tests/it/sessions/server_certificates`;
//! the certificate summary in `trust`.

mod vnc;
mod x224;
