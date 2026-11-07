//! Unit tests of the pure parts of reading what a device presents: the X.224 exchange, the
//! certificate summary and the VNC banner. The readers and `acceptServerCertificate` are
//! tested through GraphQL in `tests/it/sessions/server_certificates`.

mod certificate;
mod vnc;
mod x224;
