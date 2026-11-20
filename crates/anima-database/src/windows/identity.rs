//! A per-database TLS key and bootstrap secret, written only inside a private directory.
use crate::{Password, owner_only};
use std::{
    io::{self, Read, Write},
    path::Path,
};
use zeroize::Zeroizing;

pub(super) fn write(folder: &Path, port: u16) -> io::Result<()> {
    let made = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()])
        .map_err(io::Error::other)?;
    let key = Zeroizing::new(made.signing_key.serialize_pem());
    let password = Password::random().map_err(io::Error::other)?;
    for (name, bytes) in [
        ("server.crt", made.cert.pem().into_bytes()),
        ("port", port.to_string().into_bytes()),
    ] {
        file(&folder.join(name), &bytes)?;
    }
    let passfile = Zeroizing::new(format!("*:*:*:anima_super:{}\n", password.expose()));
    file(&folder.join("superuser.pgpass"), passfile.as_bytes())?;
    file(&folder.join("server.key"), key.as_bytes())?;
    file(
        &folder.join("superuser.password"),
        password.expose().as_bytes(),
    )
}

fn file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = owner_only::new_file(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

pub(super) fn text(path: &Path) -> io::Result<Zeroizing<String>> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(16_385));
    anima_windows::files::read(path, true)?
        .take(16_385)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 16_384 {
        return Err(io::ErrorKind::FileTooLarge.into());
    }
    let text = std::str::from_utf8(&bytes).map_err(io::Error::other)?;
    Ok(Zeroizing::new(text.to_owned()))
}
