//! The Linux key store: `keys` in the daemon's folder, beside `token`, mode 0600, replaced by
//! rename (K-E5 Section 2.1).

use std::io;
use std::path::PathBuf;

use zeroize::Zeroizing;

use super::KeyStoreError;
use crate::config::home::Home;
use crate::config::private_file::{self, PrivateFileError};

const FILE: &str = "keys";

pub struct Backend {
    home: Home,
    path: PathBuf,
}

impl Backend {
    pub fn open(home: &Home) -> Self {
        Self {
            home: home.clone(),
            path: home.dir().join(FILE),
        }
    }

    pub fn read(&self, limit: usize) -> Result<Option<Zeroizing<Vec<u8>>>, KeyStoreError> {
        self.home.ensure()?;
        Ok(private_file::read_within(&self.path, limit)?)
    }

    pub fn write(&self, bytes: &[u8]) -> Result<(), KeyStoreError> {
        self.home.ensure()?;
        private_file::write(self.home.dir(), FILE, bytes)?;
        Ok(())
    }

    pub fn delete(&self) -> Result<(), KeyStoreError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(PrivateFileError::Io(error).into()),
        }
    }
}
