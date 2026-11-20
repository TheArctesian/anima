//! Behaviour tests of the gateway, through its public interface.

mod drawn;
mod frames;
mod handshake;
mod rdp;
mod screen;
mod session;
mod view;
mod vnc;

/// A target-side override lets cross-compiled tests start the real Windows executable.
fn program() -> std::path::PathBuf {
    std::env::var_os("ANIMA_GATEWAY_EXE").map_or_else(
        || std::path::PathBuf::from(env!("CARGO_BIN_EXE_anima-gateway")),
        std::path::PathBuf::from,
    )
}
