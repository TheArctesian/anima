//! The Linux token store: `token` in the agent's folder, mode 0600, replaced by rename.

use std::path::PathBuf;

use zeroize::Zeroizing;

use super::TokenStoreError;
use crate::config::home::Home;
use crate::config::private_file;

pub struct Backend {
    home: Home,
    path: PathBuf,
}

impl Backend {
    pub fn open(home: &Home) -> Self {
        Self {
            home: home.clone(),
            path: home.token_path(),
        }
    }

    pub fn read(&self) -> Result<Option<Zeroizing<Vec<u8>>>, TokenStoreError> {
        Ok(private_file::read(&self.path)?)
    }

    pub fn write(&self, bytes: &[u8]) -> Result<(), TokenStoreError> {
        self.home.ensure()?;
        private_file::write(self.home.dir(), "token", bytes)?;
        Ok(())
    }
}
