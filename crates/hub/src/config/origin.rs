//! The public origin, accepted only in the one spelling a browser sends in `Origin` (D-3).
//!
//! Rules: scheme `https`, or `http` only for a loopback host (`localhost`, `127.0.0.0/8`,
//! `[::1]`) so tests can run without TLS; a lowercase host; a port only when it is not the
//! scheme default and has no leading zeros; nothing else (no userinfo, path, query, fragment
//! or trailing slash).

use std::net::{Ipv4Addr, Ipv6Addr};

use super::Error;

/// `text` unchanged when it is a canonical origin.
pub(super) fn canonical(variable: &'static str, text: &str) -> Result<String, Error> {
    match check(text) {
        Ok(()) => Ok(text.to_owned()),
        Err(reason) => Err(Error::Origin {
            variable,
            value: text.to_owned(),
            reason,
        }),
    }
}

fn check(text: &str) -> Result<(), &'static str> {
    let (secure, authority) = if let Some(rest) = text.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = text.strip_prefix("http://") {
        (false, rest)
    } else {
        return Err("the scheme must be https");
    };
    if authority.contains(['/', '?', '#', '@']) || authority.contains(char::is_whitespace) {
        return Err("it must have no userinfo, path, query, fragment or trailing slash");
    }
    let (host, port) = split_port(authority)?;
    check_host(host)?;
    if !secure && !is_loopback(host) {
        return Err("http is allowed only for a loopback host");
    }
    match port {
        None => Ok(()),
        Some(port) if is_default_port(secure, port) => {
            Err("the port is the scheme default and must be left out")
        }
        Some(port) => check_port(port),
    }
}

fn split_port(authority: &str) -> Result<(&str, Option<&str>), &'static str> {
    if authority.starts_with('[') {
        let end = authority.find(']').ok_or("the IPv6 host is not closed")?;
        let (host, rest) = authority.split_at(end + 1);
        return match rest {
            "" => Ok((host, None)),
            _ => rest
                .strip_prefix(':')
                .map(|port| (host, Some(port)))
                .ok_or("junk after the IPv6 host"),
        };
    }
    Ok(authority
        .rsplit_once(':')
        .map_or((authority, None), |(host, port)| (host, Some(port))))
}

fn check_host(host: &str) -> Result<(), &'static str> {
    if let Some(inner) = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        let address: Ipv6Addr = inner.parse().map_err(|_| "the IPv6 host is invalid")?;
        return if address.to_string() == inner {
            Ok(())
        } else {
            Err("the IPv6 host must be in its shortest lowercase form")
        };
    }
    let label_ok = |label: &str| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    if host.split('.').all(label_ok) {
        Ok(())
    } else {
        Err("the host must be lowercase letters, digits, hyphens and dots")
    }
}

fn is_loopback(host: &str) -> bool {
    host == "localhost"
        || host == "[::1]"
        || host
            .parse::<Ipv4Addr>()
            .is_ok_and(|address| address.is_loopback())
}

fn is_default_port(secure: bool, port: &str) -> bool {
    port == if secure { "443" } else { "80" }
}

fn check_port(port: &str) -> Result<(), &'static str> {
    match port.parse::<u16>() {
        Ok(number) if number > 0 && number.to_string() == port => Ok(()),
        _ => Err("the port must be a number from 1 to 65535 without leading zeros"),
    }
}
