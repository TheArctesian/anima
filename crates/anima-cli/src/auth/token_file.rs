//! `tokens.json` in the configuration folder, mode `0600`: one token for each hub origin, for a
//! computer with no key store (CMD-012). Separate from `config.json`, which holds no secret.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use super::token_store::StoreError;
use crate::hub::{Origin, Token};
use crate::private_file;

const FILE_NAME: &str = "tokens.json";

#[derive(Clone, Debug)]
pub struct TokenFile {
    dir: PathBuf,
}

/// Tokens by hub origin. Not `Debug`: it holds secrets.
struct Tokens(BTreeMap<String, String>);

impl TokenFile {
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_owned(),
        }
    }

    fn path(&self) -> PathBuf {
        self.dir.join(FILE_NAME)
    }

    /// The token kept for `origin`.
    ///
    /// # Errors
    /// [`StoreError`] when the file is exposed, unreadable or not a token file.
    pub fn read(&self, origin: &Origin) -> Result<Option<Token>, StoreError> {
        let mut tokens = self.load()?;
        let Some(text) = tokens.0.remove(&origin.as_text()) else {
            return Ok(None);
        };
        Token::new(Zeroizing::new(text))
            .map(Some)
            .ok_or(StoreError::BadToken)
    }

    /// Keeps `token` for `origin`, replacing an earlier one.
    ///
    /// # Errors
    /// [`StoreError`] when the file cannot be read or written.
    pub fn write(&self, origin: &Origin, token: &Token) -> Result<(), StoreError> {
        let mut tokens = self.load()?;
        tokens.0.insert(origin.as_text(), token.expose().to_owned());
        self.save(&tokens)
    }

    /// Forgets the token for `origin`; whether there was one. The file goes when it is empty.
    ///
    /// # Errors
    /// [`StoreError`] when the file cannot be read or written.
    pub fn delete(&self, origin: &Origin) -> Result<bool, StoreError> {
        let mut tokens = self.load()?;
        let Some(old) = tokens.0.remove(&origin.as_text()) else {
            return Ok(false);
        };
        drop(Zeroizing::new(old));
        if tokens.0.is_empty() {
            private_file::remove(&self.path()).map_err(|source| self.failed(source))?;
            return Ok(true);
        }
        self.save(&tokens).map(|()| true)
    }

    fn load(&self) -> Result<Tokens, StoreError> {
        let path = self.path();
        let Some(bytes) = private_file::read(&path).map_err(|source| self.failed(source))? else {
            return Ok(Tokens(BTreeMap::new()));
        };
        // The parser's message could quote the file, so only "does not read" is kept.
        serde_json::from_slice(&bytes)
            .map(Tokens)
            .map_err(|_| StoreError::Corrupt { path })
    }

    fn save(&self, tokens: &Tokens) -> Result<(), StoreError> {
        let bytes = Zeroizing::new(serde_json::to_vec(&tokens.0).expect("tokens serialise"));
        private_file::write(&self.dir, FILE_NAME, &bytes).map_err(|source| self.failed(source))
    }

    fn failed(&self, source: private_file::PrivateFileError) -> StoreError {
        StoreError::File {
            path: self.path(),
            source,
        }
    }
}

impl Drop for Tokens {
    fn drop(&mut self) {
        for (_, token) in std::mem::take(&mut self.0) {
            drop(Zeroizing::new(token));
        }
    }
}
