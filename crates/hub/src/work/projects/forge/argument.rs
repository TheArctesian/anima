//! A request the provider would not build because of an argument the caller gave (a file path
//! or a ref): `VALIDATION` naming that argument, with fixed words, before anything is sent
//! (K-E1 Section 13.8). Never the path or the ref itself (LOG-007). Pure.

use async_graphql::Result;
use source_exchange::{Problem, Request};

use crate::api_errors::{FieldError, invalid};

/// The provider's codes for a path and a ref it refused (`provider_github::live`).
const PATH_REFUSED: &str = "path_unusable";
const REF_REFUSED: &str = "ref_unusable";

/// `request` unchanged, unless the provider refused the caller's path or ref: then one
/// `VALIDATION` error naming it. Any other refusal (the repository's stored name) passes on for
/// [`super::read`] to answer.
///
/// # Errors
/// `VALIDATION` for a refused `path` or `ref`.
pub fn checked(request: Result<Request, Problem>) -> Result<Result<Request, Problem>> {
    match &request {
        Err(problem) if problem.code == PATH_REFUSED => Err(invalid(&[FieldError::new(
            "path",
            "Give a path from the repository root of at most 1,024 bytes and 40 parts: no \
             leading slash, no empty, . or .. part, no backslash and no control character.",
        )])),
        Err(problem) if problem.code == REF_REFUSED => Err(invalid(&[FieldError::new(
            "ref",
            "Give a branch name or a 40-digit commit of at most 255 bytes: no space, no \
             control character, no .. and no leading dash.",
        )])),
        _ => Ok(request),
    }
}
