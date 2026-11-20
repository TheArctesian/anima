//! The Windows database transport: IPv4 loopback with a private, pinned TLS identity.
//! `SQLx` has no Windows Unix-socket connector. TLS authenticates the server before any
//! password is sent, so another local process cannot impersonate a stopped database.
mod identity;

use crate::{Layout, Password};
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::{
    fs, io,
    net::TcpListener,
    path::{Path, PathBuf},
};

pub(crate) struct Endpoint {
    folder: PathBuf,
    port: u16,
}

impl Endpoint {
    /// Create once, publishing the whole private identity directory atomically.
    pub(crate) fn create(layout: &Layout) -> io::Result<Self> {
        let folder = layout.root().join("connection");
        if folder.exists() {
            return Self::load(layout);
        }
        let temporary = tempfile::Builder::new()
            .prefix(".connection-")
            .tempdir_in(layout.root())?;
        anima_windows::files::protect(temporary.path())?;
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        // This reservation is released before PostgreSQL starts. If another process takes
        // it, startup fails; clients still require our TLS identity before authentication.
        identity::write(temporary.path(), port)?;
        fs::rename(temporary.path(), &folder)?;
        Self::load(layout)
    }

    pub(crate) fn load(layout: &Layout) -> io::Result<Self> {
        let folder = layout.root().join("connection");
        if !fs::symlink_metadata(&folder)?.is_dir() {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        anima_windows::files::directory(&folder)?;
        if !anima_windows::files::secure(&folder, true)? {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let port = identity::text(&folder.join("port"))?
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or(io::ErrorKind::InvalidData)?;
        for name in [
            "server.crt",
            "server.key",
            "superuser.password",
            "superuser.pgpass",
        ] {
            let _ = anima_windows::files::read(&folder.join(name), true)?;
        }
        Ok(Self { folder, port })
    }

    pub(crate) fn password_file(&self) -> PathBuf {
        self.folder.join("superuser.password")
    }

    pub(crate) fn password(&self) -> io::Result<Password> {
        let text = identity::text(&self.password_file())?;
        let text = text.trim();
        if text.len() != 48 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(Password::from_text(text.to_owned()))
    }

    pub(crate) fn client_command(
        &self,
        tree: &crate::Tree,
        program: crate::Program,
    ) -> std::process::Command {
        let mut command = tree.command(program);
        command
            .args(["--host", "127.0.0.1", "--port"])
            .arg(self.port.to_string())
            .env("PGSSLMODE", "verify-full")
            .env("PGSSLROOTCERT", self.certificate())
            .env("PGPASSFILE", self.folder.join("superuser.pgpass"));
        command
    }

    pub(crate) fn options(&self) -> PgConnectOptions {
        PgConnectOptions::new_without_pgpass()
            .host("127.0.0.1")
            .port(self.port)
            .ssl_mode(PgSslMode::VerifyFull)
            .ssl_root_cert(self.folder.join("server.crt"))
    }

    pub(crate) fn settings(&self) -> io::Result<Vec<String>> {
        Ok(vec![
            "listen_addresses=127.0.0.1".into(),
            "unix_socket_directories=".into(),
            format!("port={}", self.port),
            "ssl=on".into(),
            "ssl_min_protocol_version=TLSv1.2".into(),
            format!(
                "ssl_cert_file={}",
                path_text(&self.folder.join("server.crt"))?
            ),
            format!(
                "ssl_key_file={}",
                path_text(&self.folder.join("server.key"))?
            ),
        ])
    }

    pub(crate) fn authority(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    pub(crate) fn certificate(&self) -> PathBuf {
        self.folder.join("server.crt")
    }
}

fn path_text(path: &Path) -> io::Result<String> {
    path.to_str()
        .map(|text| text.replace('\\', "/"))
        .ok_or(io::ErrorKind::InvalidData.into())
}

#[cfg(test)]
mod tests;
