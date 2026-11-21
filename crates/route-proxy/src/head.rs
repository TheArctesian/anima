//! The head of one HTTP/1.1 request from the internet, read and written again before it goes to
//! the application (Section 12.3, SF-NEW-routeheaders). Pure.
//!
//! The proxy removes each `Forwarded`, `X-Forwarded-*` and `X-Real-IP` header that the client
//! sent and each header of the hub or the door, then sets `X-Forwarded-For` and
//! `X-Forwarded-Proto` from what it knows. It removes each cookie of a hub. The `Host` must be
//! the name of the TLS SNI field, or the answer is 421. One request goes on each connection to
//! the application (`Connection: close`), so each request of the client is read here; a body
//! must have one `Content-Length` of digits only, and never with a `Transfer-Encoding`; a header
//! name with `_` is refused, so that no application reads it as a name with `-`
//! (SF-NEW-revrouteheaders). A WebSocket upgrade with its `Sec-WebSocket-Key` keeps its
//! `Connection: upgrade`; without the key the request is a plain one (SF-NEW-revrouteupgrade).

use std::net::IpAddr;

/// The longest head that the proxy reads.
pub const HEAD_MAX: usize = 16 * 1024;
/// The most header lines.
const HEADERS_MAX: usize = 100;
/// Headers that the client may not set: the proxy sets them, or the hub or the door uses them.
const DROPPED: [&str; 7] = [
    "forwarded",
    "x-real-ip",
    "connection",
    "keep-alive",
    "proxy-connection",
    "te",
    "trailer",
];
/// Prefixes of headers that the client may not set.
const DROPPED_PREFIXES: [&str; 2] = ["x-forwarded-", "x-anima-"];
/// The prefixes of the names of the cookies of a hub (`__Host-anima`, R-111).
const HUB_COOKIES: [&str; 2] = ["__host-anima", "anima_"];

/// An answer of the proxy itself, in place of the application's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    BadRequest = 400,
    RequestTimeout = 408,
    LengthRequired = 411,
    Misdirected = 421,
    HeadTooLarge = 431,
    NotImplemented = 501,
}

impl Status {
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }

    const fn reason(self) -> &'static str {
        match self {
            Self::BadRequest => "Bad Request",
            Self::RequestTimeout => "Request Timeout",
            Self::LengthRequired => "Length Required",
            Self::Misdirected => "Misdirected Request",
            Self::HeadTooLarge => "Request Header Fields Too Large",
            Self::NotImplemented => "Not Implemented",
        }
    }

    /// The whole answer, with no body of an application.
    #[must_use]
    pub fn answer(self) -> Vec<u8> {
        format!(
            "HTTP/1.1 {} {}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
            self.code(),
            self.reason()
        )
        .into_bytes()
    }
}

/// A request head that passed [`rewrite`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rewritten {
    /// The head to send to the application.
    pub head: Vec<u8>,
    /// The bytes of the body that follow the head.
    pub body: u64,
    /// A WebSocket upgrade with its key: after an answer `101`, the bytes go both ways as they
    /// are.
    pub upgrade: bool,
    /// The path of the request, for the redirect of port 80 only. Never logged.
    pub target: String,
}

/// The end of the head in `bytes`: the index after `\r\n\r\n`.
#[must_use]
pub fn end_of_head(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| at + 4)
}

/// Writes the head again for the application of `sni`, for the client at `client`.
///
/// # Errors
/// The [`Status`] that the proxy answers in place of the application.
pub fn rewrite(head: &[u8], sni: &str, client: IpAddr) -> Result<Rewritten, Status> {
    let text = std::str::from_utf8(head).map_err(|_| Status::BadRequest)?;
    let mut lines = text.split("\r\n").filter(|line| !line.is_empty());
    let request_line = lines.next().ok_or(Status::BadRequest)?;
    let target = request_target(request_line)?;
    let mut kept = vec![request_line.to_owned()];
    let mut framing = Framing::default();
    for (count, line) in lines.enumerate() {
        if count >= HEADERS_MAX {
            return Err(Status::HeadTooLarge);
        }
        let (name, value) = header(line)?;
        let lower = name.to_ascii_lowercase();
        if lower == "cookie" {
            if let Some(cookies) = without_hub_cookies(value) {
                kept.push(format!("{name}: {cookies}"));
            }
            continue;
        }
        framing.note(&lower, value)?;
        if !dropped(&lower) {
            kept.push(format!("{name}: {value}"));
        }
    }
    if !framing
        .host
        .as_deref()
        .is_some_and(|host| same_host(host, sni))
    {
        return Err(Status::Misdirected);
    }
    let (body, upgrade) = framing.body()?;
    kept.push(format!("x-forwarded-for: {client}"));
    kept.push("x-forwarded-proto: https".to_owned());
    kept.push(
        if upgrade {
            "connection: upgrade"
        } else {
            "connection: close"
        }
        .to_owned(),
    );
    let mut head = kept.join("\r\n");
    head.push_str("\r\n\r\n");
    Ok(Rewritten {
        head: head.into_bytes(),
        body,
        upgrade,
        target,
    })
}

/// What the headers of a request say of its host, its body and an upgrade.
#[derive(Default)]
struct Framing {
    host: Option<String>,
    length: Option<u64>,
    chunked: bool,
    websocket: bool,
    key: bool,
}

impl Framing {
    /// Notes one header by its name in lower case.
    fn note(&mut self, lower: &str, value: &str) -> Result<(), Status> {
        match lower {
            "host" | "content-length" if self.seen(lower) => return Err(Status::BadRequest),
            "host" => self.host = Some(value.to_owned()),
            "content-length" => self.length = Some(content_length(value)?),
            "transfer-encoding" => self.chunked = true,
            "upgrade" => self.websocket = value.eq_ignore_ascii_case("websocket"),
            "sec-websocket-key" => self.key = !value.is_empty(),
            _ => {}
        }
        Ok(())
    }

    fn seen(&self, lower: &str) -> bool {
        if lower == "host" {
            self.host.is_some()
        } else {
            self.length.is_some()
        }
    }

    /// The length of the body and whether the request is a WebSocket upgrade with its key.
    fn body(&self) -> Result<(u64, bool), Status> {
        match (self.chunked, self.length) {
            (true, Some(_)) => Err(Status::BadRequest),
            (true, None) => Err(Status::LengthRequired),
            (false, length) => {
                let body = length.unwrap_or(0);
                let upgrade = self.websocket && self.key;
                if upgrade && body > 0 {
                    return Err(Status::BadRequest);
                }
                Ok((body, upgrade))
            }
        }
    }
}

/// The target of `METHOD target HTTP/1.x`, which must be a path.
fn request_target(line: &str) -> Result<String, Status> {
    let mut parts = line.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Status::BadRequest);
    };
    if !method.bytes().all(|b| b.is_ascii_uppercase()) || method.is_empty() {
        return Err(Status::BadRequest);
    }
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(Status::NotImplemented);
    }
    if !target.starts_with('/') {
        return Err(Status::BadRequest);
    }
    Ok(target.to_owned())
}

/// A `Content-Length` value: digits only, with no sign, space or list.
fn content_length(value: &str) -> Result<u64, Status> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Status::BadRequest);
    }
    value.parse().map_err(|_| Status::BadRequest)
}

/// One header line as name and value; a folded line or a bad name is refused. A name is a
/// token of RFC 9110 without `_`: a server that reads `_` as `-` would take `X_Forwarded_For`
/// for a header that the proxy removes.
fn header(line: &str) -> Result<(&str, &str), Status> {
    let (name, value) = line.split_once(':').ok_or(Status::BadRequest)?;
    let token = |b: u8| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^`|~".contains(&b);
    if name.is_empty() || !name.bytes().all(token) {
        return Err(Status::BadRequest);
    }
    let value = value.trim_matches([' ', '\t']);
    if value.bytes().any(|b| b == 0 || b == b'\r' || b == b'\n') {
        return Err(Status::BadRequest);
    }
    Ok((name, value))
}

fn dropped(lower: &str) -> bool {
    DROPPED.contains(&lower)
        || DROPPED_PREFIXES
            .iter()
            .any(|prefix| lower.starts_with(prefix))
}

/// The `Cookie` value without the cookies of a hub; `None` when none is left.
fn without_hub_cookies(value: &str) -> Option<String> {
    let kept: Vec<&str> = value
        .split(';')
        .map(str::trim)
        .filter(|pair| !pair.is_empty())
        .filter(|pair| {
            let name = pair
                .split('=')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            !HUB_COOKIES.iter().any(|prefix| name.starts_with(prefix))
        })
        .collect();
    (!kept.is_empty()).then(|| kept.join("; "))
}

/// Whether the `Host` header names `sni`, with no port or port 443.
fn same_host(host: &str, sni: &str) -> bool {
    let name = host.strip_suffix(":443").unwrap_or(host);
    let name = name.strip_suffix('.').unwrap_or(name);
    name.eq_ignore_ascii_case(sni)
}
