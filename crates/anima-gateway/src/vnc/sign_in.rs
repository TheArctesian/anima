//! The opening of a VNC connection (RFC 6143 Sections 7.1 to 7.3): the version, the
//! security type, the sign-in and the two init messages. Sign-in methods: none, VNC
//! authentication (type 2) and Apple's (type 30, what macOS Screen Sharing uses for an
//! account).
//!
//! In two steps, because the gateway may have to ask the operator for a credential once it
//! knows which method the device wants (GW-015). I/O edge: the caller sets the time limit.

use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};

use super::auth::{self, AppleChallenge, KEY_BYTES, NoAppleAnswer};
use super::requests;
use crate::handshake::Credential;
use crate::screen::MAX_SIDE;

const NONE: u8 = 1;
const VNC_PASSWORD: u8 = 2;
const APPLE: u8 = 30;
/// The newest standard version. macOS announces 003.889 and accepts this answer.
const VERSION: &[u8; 12] = b"RFB 003.008\n";
/// The longest reason text read from a device; it is read and not kept.
const LONGEST_REASON: u32 = 4096;
/// The longest desktop name read from a device; it is read and not kept.
const LONGEST_NAME: u32 = 1 << 16;

/// Why the sign-in did not finish. No variant holds a value from the device or an account
/// (LOG-007).
#[derive(Debug, thiserror::Error)]
pub enum SignInError {
    #[error("the connection to the device failed")]
    Io(#[from] std::io::Error),
    #[error("the device did not answer as a VNC server of version 3.8 or later")]
    NotVnc,
    #[error("the device refused the connection before the sign-in")]
    Turned,
    #[error("the device offers no sign-in method the gateway has")]
    NoMethod,
    #[error("the device refused the sign-in")]
    Refused,
    #[error("the account name or password is too long for the device's method")]
    TooLong,
    #[error("the device's sign-in numbers or screen size cannot be used")]
    BadNumbers,
}

/// A sign-in method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    None,
    VncPassword,
    Apple,
}

impl Method {
    /// What the method needs to sign in.
    #[must_use]
    pub const fn needs(self) -> &'static [Credential] {
        match self {
            Self::None => &[],
            Self::VncPassword => &[Credential::Password],
            Self::Apple => &[Credential::Username, Credential::Password],
        }
    }

    const fn number(self) -> u8 {
        match self {
            Self::None => NONE,
            Self::VncPassword => VNC_PASSWORD,
            Self::Apple => APPLE,
        }
    }
}

/// The security types a device offers.
#[derive(Clone, Debug)]
pub struct Offered(Vec<u8>);

impl Offered {
    /// The method to use for what the hub has.
    ///
    /// - With an account name: Apple's method only. The account's password is not sent by
    ///   the weaker VNC password method, even when a device (or someone between) offers
    ///   only that.
    /// - Without one: a VNC password, else Apple's method (the name is then asked for).
    /// - With no credential at all: no sign-in when the device offers that, else a method
    ///   whose credentials are then asked for.
    /// - No sign-in is used only then. A device that suddenly
    ///   asks for none, while the hub holds a password for it, is not the device meant.
    ///
    /// # Errors
    /// [`SignInError::NoMethod`] when the device offers no method that may be used.
    pub fn method(&self, has_username: bool, has_password: bool) -> Result<Method, SignInError> {
        let has = |method: Method| self.0.contains(&method.number());
        let order: &[Method] = match (has_username, has_password) {
            (true, _) => &[Method::Apple],
            (false, true) => &[Method::VncPassword, Method::Apple],
            (false, false) => &[Method::None, Method::VncPassword, Method::Apple],
        };
        order
            .iter()
            .copied()
            .find(|method| has(*method))
            .ok_or(SignInError::NoMethod)
    }
}

/// An account to sign in with. Either part may be empty when the method does not use it.
#[derive(Clone, Copy)]
pub struct Account<'a> {
    pub username: &'a str,
    pub password: &'a str,
}

/// Exchanges versions and reads the security types the device offers.
///
/// # Errors
/// [`SignInError`] when the peer is not a VNC server of version 3.8 or later, or refuses.
pub async fn offered<S>(device: &mut S) -> Result<Offered, SignInError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut banner = [0; 12];
    device.read_exact(&mut banner).await?;
    let version = banner
        .strip_prefix(b"RFB 003.")
        .and_then(|rest| rest.strip_suffix(b"\n"));
    let minor = version.filter(|minor| minor.iter().all(u8::is_ascii_digit));
    if minor.is_none_or(|minor| minor < b"008".as_slice()) {
        return Err(SignInError::NotVnc);
    }
    device.write_all(VERSION).await?;
    let count = device.read_u8().await?;
    if count == 0 {
        skip(device, LONGEST_REASON).await?;
        return Err(SignInError::Turned);
    }
    let mut types = vec![0; usize::from(count)];
    device.read_exact(&mut types).await?;
    Ok(Offered(types))
}

/// Signs in with `method`, completes the init exchange and asks for the gateway's pixel
/// format and encodings. Returns the device's screen size. `random` fills a buffer with
/// random bytes.
///
/// # Errors
/// [`SignInError`]; `Refused` when the device does not accept the account.
pub async fn sign_in<S>(
    device: &mut S,
    method: Method,
    account: Account<'_>,
    random: &mut (dyn FnMut(&mut [u8]) + Send),
) -> Result<(usize, usize), SignInError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    device.write_u8(method.number()).await?;
    match method {
        Method::None => {}
        Method::VncPassword => {
            let mut challenge = [0; 16];
            device.read_exact(&mut challenge).await?;
            let answer = auth::vnc_password_answer(challenge, account.password);
            device.write_all(&answer).await?;
        }
        Method::Apple => apple(device, account, random).await?,
    }
    if device.read_u32().await? != 0 {
        // Version 3.8 sends a reason; Apple's server closes without one.
        let _ = skip(device, LONGEST_REASON).await;
        return Err(SignInError::Refused);
    }
    device.write_u8(1).await?; // ClientInit: share the screen with other viewers
    let (width, height) = (device.read_u16().await?, device.read_u16().await?);
    let mut pixel_format = [0; 16];
    device.read_exact(&mut pixel_format).await?;
    skip(device, LONGEST_NAME).await?;
    let sides = 1..=MAX_SIDE;
    let size = (usize::from(width), usize::from(height));
    if !sides.contains(&size.0) || !sides.contains(&size.1) {
        return Err(SignInError::BadNumbers);
    }
    device.write_all(&requests::pixel_format()).await?;
    device.write_all(&requests::encodings()).await?;
    Ok(size)
}

/// Apple's method: reads the device's numbers and sends the answer.
async fn apple<S>(
    device: &mut S,
    account: Account<'_>,
    random: &mut (dyn FnMut(&mut [u8]) + Send),
) -> Result<(), SignInError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let generator = device.read_u16().await?;
    let length = usize::from(device.read_u16().await?);
    if !KEY_BYTES.contains(&length) {
        return Err(SignInError::BadNumbers);
    }
    let mut challenge = AppleChallenge {
        generator,
        prime: vec![0; length],
        server_public: vec![0; length],
    };
    device.read_exact(&mut challenge.prime).await?;
    device.read_exact(&mut challenge.server_public).await?;
    let mut private = zeroize::Zeroizing::new(vec![0; length]);
    random(&mut private);
    let mut filler = [0; 128];
    random(&mut filler);
    let answer = auth::apple_answer(
        &challenge,
        (account.username, account.password),
        &private,
        filler,
    )
    .map_err(|error| match error {
        NoAppleAnswer::TooLong => SignInError::TooLong,
        NoAppleAnswer::BadNumbers => SignInError::BadNumbers,
    })?;
    device.write_all(&answer).await?;
    Ok(())
}

/// Reads a text that begins with its length and drops it. A length over `longest` is not
/// a text a VNC server sends.
async fn skip<S: AsyncRead + Unpin>(device: &mut S, longest: u32) -> Result<(), SignInError> {
    let length = device.read_u32().await?;
    if length > longest {
        return Err(SignInError::BadNumbers);
    }
    let mut text = device.take(u64::from(length));
    tokio::io::copy(&mut text, &mut tokio::io::sink()).await?;
    Ok(())
}
