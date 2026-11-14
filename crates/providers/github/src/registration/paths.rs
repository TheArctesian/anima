//! The pages and API paths of the flow. GitHub's slugs, logins and codes are untrusted text:
//! each goes into a path only when it is plain, and otherwise the path is a fixed page that
//! names nothing from the reply.

use source_exchange::Owner;

const APPS_LIST: &str = "/settings/apps";
const INSTALLATIONS_LIST: &str = "/settings/installations";
/// A path the API answers with "not found": what a code or id that is not plain becomes.
const NO_SUCH_CODE: &str = "/app-manifests/invalid/conversions";
const NO_SUCH_INSTALLATION: &str = "/app/installations/0";

/// A GitHub login: letters, digits and single hyphens, 1 to 39 characters, no hyphen at
/// either end.
pub(crate) fn is_login(text: &str) -> bool {
    (1..=39).contains(&text.len())
        && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        && !text.starts_with('-')
        && !text.ends_with('-')
        && !text.contains("--")
}

/// An App slug: letters, digits, hyphens and underscores, 1 to 64 characters.
pub(crate) fn is_slug(text: &str) -> bool {
    (1..=64).contains(&text.len())
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// `/settings/apps/new`, or the organisation's. `None` for an organisation that is not a login.
pub(crate) fn form_path(organisation: Option<&str>) -> Option<String> {
    match organisation {
        None => Some("/settings/apps/new".to_owned()),
        Some(login) if is_login(login) => Some(format!("/organizations/{login}/settings/apps/new")),
        Some(_) => None,
    }
}

/// Where the operator presses Install. Not a plain slug: the list of the account's installations.
pub(crate) fn install_path(slug: &str) -> String {
    if is_slug(slug) {
        format!("/apps/{slug}/installations/new")
    } else {
        INSTALLATIONS_LIST.to_owned()
    }
}

/// The App's advanced settings, where it can be deleted; the list of Apps without a plain slug.
pub(crate) fn settings_path(owner: Option<&Owner>, slug: Option<&str>) -> String {
    let base = match owner {
        Some(owner) if owner.organisation && is_login(&owner.login) => {
            format!("/organizations/{}{APPS_LIST}", owner.login)
        }
        _ => APPS_LIST.to_owned(),
    };
    match slug {
        Some(slug) if is_slug(slug) => format!("{base}/{slug}/advanced"),
        _ => base,
    }
}

/// The conversion path. `code` is plain (letters, digits, `-`, `_`, 1 to 200) or the path names
/// a code that does not exist.
pub(crate) fn conversion_path(code: &str) -> String {
    if (1..=200).contains(&code.len())
        && code
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        format!("/app-manifests/{code}/conversions")
    } else {
        NO_SUCH_CODE.to_owned()
    }
}

/// One installation of the App. `id` is 1 to 20 digits or the path names an installation that
/// does not exist.
pub(crate) fn installation_path(id: &str) -> String {
    if (1..=20).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit()) {
        format!("/app/installations/{id}")
    } else {
        NO_SUCH_INSTALLATION.to_owned()
    }
}
