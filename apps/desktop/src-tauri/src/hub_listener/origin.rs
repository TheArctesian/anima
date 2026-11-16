//! A loopback hub origin as the desktop client saves it (F-1): `http://127.0.0.1:<port>` and
//! nothing more. `localhost` and `[::1]` are refused: a desktop loopback hub is on
//! `127.0.0.1` only, so one address is checked. Pure.

/// The port of the origin, or none when it is not `http://127.0.0.1:<port>`.
pub fn port(origin: &str) -> Option<u16> {
    let port = origin.strip_prefix("http://127.0.0.1:")?;
    let digits =
        !port.is_empty() && port.len() <= 5 && port.bytes().all(|byte| byte.is_ascii_digit());
    let port: u16 = port.parse().ok().filter(|_| digits)?;
    (port != 0).then_some(port)
}
