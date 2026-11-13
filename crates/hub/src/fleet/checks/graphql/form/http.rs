//! The HTTP configuration input as domain values: headers, authentication and status ranges,
//! with the secrets sealed apart from the stored configuration (LOG-007, SEC-005).

use super::super::input::{AuthShape, HttpShape};
use super::{ConfigForm, number, or_default};
use crate::fleet::checks::{
    Definition, FieldError,
    config::{HttpCredentials, HttpInput, StatusRange, is_secret_header},
};

pub(super) fn http(shape: HttpShape) -> Result<ConfigForm, FieldError> {
    let mut sealed = false;
    let mut headers = Vec::new();
    for header in shape.headers.unwrap_or_default() {
        let header = header.0;
        sealed |= is_secret_header(&header.name) && !header.value.0.expose().is_empty();
        headers.push((header.name, header.value.0));
    }
    let credentials = match shape.auth.map(|auth| auth.0) {
        None => HttpCredentials::None,
        Some(AuthShape::Basic(basic)) => {
            let basic = basic.0;
            sealed |= !basic.password.0.expose().is_empty();
            HttpCredentials::Basic {
                username: basic.username,
                password: basic.password.0,
            }
        }
        Some(AuthShape::BearerToken(token)) => {
            sealed |= !token.0.expose().is_empty();
            HttpCredentials::Bearer(token.0)
        }
    };
    let ranges = shape.accepted_status.unwrap_or_default();
    let accepted_status = ranges
        .into_iter()
        .map(|range| {
            Ok(StatusRange {
                from: number("acceptedStatus", range.from)?,
                to: number("acceptedStatus", range.to)?,
            })
        })
        .collect::<Result<_, FieldError>>()?;
    let input = HttpInput {
        url: shape.url,
        method: shape.method.unwrap_or_default().into(),
        headers,
        body: shape.body,
        max_redirects: or_default("maxRedirects", shape.max_redirects, 10)?,
        accepted_status,
        keyword: shape.keyword,
        json_query: shape.json_query,
        json_expected: shape.json_expected,
        credentials,
        verify_tls: shape.verify_tls.unwrap_or(true),
        timeout_ms: or_default("timeoutMs", shape.timeout_ms, 10_000)?,
    };
    Ok(ConfigForm {
        definition: Definition::Http(input),
        children: None,
        sealed,
    })
}
