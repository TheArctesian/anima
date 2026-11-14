//! Finding the installation of the App the attempt made, and the task that watches the attempt
//! (K-E1 Section 10a).
//!
//! An installation ID from a browser redirect is not trusted: the hub asks GitHub about it with
//! the App's own token, and takes it only when GitHub found it, it belongs to this App and to
//! the App's owner, and it is not suspended. Anything else is ignored. While the App waits for
//! Install, the watch also asks for the App's installations every few seconds, so a closed tab
//! does not stop the flow.

use ring::rand::SystemRandom;
use source_exchange::{Declaration, Installation};
use tokio::task::JoinHandle;
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{GITHUB, attempt::App, attempts::Attempts, finish::finish};
use crate::engine::sources::{
    Scheduling,
    acquisition::{
        app_key::AppSigner,
        installations::{Lookup, installation, installations},
    },
};

/// Whether `found` is an installation of `app` in its owner's account that works.
#[must_use]
pub fn fits(found: &Installation, app: &App) -> bool {
    found.app_id == app.id && found.account_id == app.owner.id && !found.suspended
}

/// The App's own signer, from the key GitHub gave it; `None` for a key that cannot sign.
fn signer(app: &App, key: &Zeroizing<String>) -> Option<AppSigner> {
    AppSigner::read(app.id, key.as_bytes()).ok()
}

/// The installation `id` from a redirect, when GitHub says it [`fits`] `app`.
pub async fn verified(
    scheduling: &Scheduling,
    (app, key): (&App, &Zeroizing<String>),
    id: &str,
) -> Option<u64> {
    let declaration: Declaration = *scheduling.registry.declaration(GITHUB)?;
    let signer = signer(app, key)?;
    let now = scheduling.clock.now();
    let random = SystemRandom::new();
    let asked = installation(
        &scheduling.transport,
        &declaration,
        (&signer, id),
        (now, &random),
    )
    .await;
    match asked {
        Lookup::Found(found) if fits(&found, app) && found.id.to_string() == id => Some(found.id),
        _ => None,
    }
}

/// The first installation of `app` that [`fits`] it, from the App's list.
async fn listed(scheduling: &Scheduling, (app, key): (&App, &Zeroizing<String>)) -> Option<u64> {
    let declaration: Declaration = *scheduling.registry.declaration(GITHUB)?;
    let signer = signer(app, key)?;
    let now = scheduling.clock.now();
    let random = SystemRandom::new();
    match installations(&scheduling.transport, &declaration, &signer, (now, &random)).await {
        Lookup::Found(all) => all
            .iter()
            .find(|found| fits(found, app))
            .map(|found| found.id),
        _ => None,
    }
}

/// Starts the task that watches attempt `id`: every poll interval, or when woken, it expires
/// the attempt when its time is over and, while the App waits for Install, asks GitHub for its
/// installations. It ends when the attempt ends or is replaced; tasks end with the hub.
pub fn watch(scheduling: Scheduling, attempts: Attempts, id: Uuid) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                () = tokio::time::sleep(attempts.poll()) => {}
                () = attempts.woken() => {}
            }
            let now = scheduling.clock.now();
            let seen = attempts.step(Some(id), now, |held| Ok((held.ended(), held.waiting_app())));
            let Ok((false, waiting)) = seen else {
                return;
            };
            let Some((app, key)) = waiting else {
                continue;
            };
            if let Some(found) = listed(&scheduling, (&app, &key)).await {
                finish(&scheduling, &attempts, (id, found)).await;
            }
        }
    })
}
