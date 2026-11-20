use serde::{Deserialize, Serialize};
use tauri::{Runtime, plugin::PluginHandle};

/// The native session store refused the operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoreError;

/// The app's native store. Plugin calls stay inside the shell.
pub struct SecureTokens<R: Runtime>(PluginHandle<R>);

#[derive(Serialize)]
struct Entry<'a> {
    scope: &'a str,
    account: &'a str,
}

#[derive(Serialize)]
struct Write<'a> {
    scope: &'a str,
    account: &'a str,
    token: &'a str,
}

#[derive(Deserialize)]
struct Read {
    token: Option<String>,
}

impl<R: Runtime> SecureTokens<R> {
    pub(super) fn new(handle: PluginHandle<R>) -> Self {
        Self(handle)
    }

    /// Reads an entry. A missing entry returns `None`.
    ///
    /// # Errors
    /// Returns an error if the native store cannot read the entry.
    pub async fn read(&self, scope: &str, account: &str) -> Result<Option<String>, StoreError> {
        self.0
            .run_mobile_plugin_async::<Read>("readToken", Entry { scope, account })
            .await
            .map(|read| read.token)
            .map_err(|_| StoreError)
    }

    /// Replaces an entry.
    ///
    /// # Errors
    /// Returns an error if the native store cannot write the entry.
    pub async fn write(&self, scope: &str, account: &str, token: &str) -> Result<(), StoreError> {
        self.0
            .run_mobile_plugin_async(
                "writeToken",
                Write {
                    scope,
                    account,
                    token,
                },
            )
            .await
            .map_err(|_| StoreError)
    }

    /// Clears an entry. Clearing an absent entry succeeds.
    ///
    /// # Errors
    /// Returns an error if the native store cannot delete the entry.
    pub async fn clear(&self, scope: &str, account: &str) -> Result<(), StoreError> {
        self.0
            .run_mobile_plugin_async("clearToken", Entry { scope, account })
            .await
            .map_err(|_| StoreError)
    }
}
