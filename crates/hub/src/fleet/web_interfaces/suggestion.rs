//! Which HTTP checks are offered as interfaces of a device (WEB-002, D-7). Pure.

use std::collections::HashSet;

use reqwest::Url;

use super::{
    model::{Suggestion, WebInterface},
    rules,
};
use crate::ids::CheckId;

/// One suggestion for each HTTP check of a device that is not dismissed, whose address is
/// not already an interface of the device (compared as parsed URLs), and whose address the
/// URL rule accepts; an address with a user name would show it (LOG-007). The label is
/// the host.
pub(super) fn filter(
    http_checks: &[(CheckId, String)],
    interfaces: &[WebInterface],
    dismissed: &HashSet<CheckId>,
) -> Vec<Suggestion> {
    let taken: Vec<Url> = interfaces
        .iter()
        .filter_map(|interface| Url::parse(&interface.url).ok())
        .collect();
    http_checks
        .iter()
        .filter(|(check, _)| !dismissed.contains(check))
        .filter_map(|(check, text)| {
            let (url, parsed) = rules::parse_url(text).ok()?;
            let label = rules::host_label(parsed.host_str()?);
            (!taken.contains(&parsed)).then_some(Suggestion {
                check: *check,
                url,
                label,
            })
        })
        .collect()
}

/// Whether two addresses are the same URL once parsed.
pub(super) fn same_url(a: &str, b: &str) -> bool {
    matches!((Url::parse(a), Url::parse(b)), (Ok(a), Ok(b)) if a == b)
}
