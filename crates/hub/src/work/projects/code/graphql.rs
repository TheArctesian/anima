//! The code roots (K1): one directory of a repository, kept a short while, and one file, never
//! kept and closed to every API key (K-E1 Section 13.8, D-23).
//!
//! Neither a path nor a file's content reaches a log line, an error message, the audit log or
//! an event (LOG-007): errors carry fixed words only, and a read writes no audit entry.

use async_graphql::{Context, ID, Object, Result};
use provider_github::live::contents::{self as github, DirectoryRead, FileRead};
use source_exchange::{Body, Response};

use super::{
    address::is_commit,
    views::{CodeDirectoryView, CodeFileContent, CodeFileView, Place, directory, file},
};
use crate::{
    api_errors::{FieldError, invalid},
    engine::sources::Keep,
    fleet::access::{Authenticated, operator_only},
    work::projects::forge::{self, Target, missing},
};

/// A directory of a branch, which can move, and of a commit, which cannot. GitHub answers a
/// directory read of a file with the file, content included: only a listing is ever kept.
const BRANCH_DIRECTORY: Keep = Keep::for_seconds(60).only_when(listing);
const COMMIT_DIRECTORY: Keep = Keep::for_seconds(3_600).only_when(listing);
/// The ref K1 shows for the default branch when the last sync read none (an empty repository).
const HEAD: &str = "HEAD";

/// Whether a contents answer is a directory listing: a JSON array. A file, a link or a
/// submodule is an object (K-E1 Section 13.1: file content is never kept).
fn listing(body: &[u8]) -> bool {
    body.trim_ascii_start().first() == Some(&b'[')
}

/// `VALIDATION` on `path` with fixed words: never the path itself.
fn wrong_kind(words: &'static str) -> async_graphql::Error {
    invalid(&[FieldError::new("path", words)])
}

/// The ref K1 shows: the one asked, or the repository's default branch.
fn shown<'a>(git_ref: Option<&'a str>, target: &'a Target) -> &'a str {
    git_ref.unwrap_or_else(|| target.default_branch.as_deref().unwrap_or(HEAD))
}

/// What `response` holds as a file: a 200 whose body was larger than the hub reads is a file
/// too large to show (decisions item 13).
fn file_read(response: &Response<'_>) -> Result<FileRead, source_exchange::Problem> {
    if matches!(response.body, Body::TooLarge) {
        return Ok(FileRead::TooLarge(None));
    }
    github::file_parse(response)
}

/// A file read as K1's file parts, or `VALIDATION` for a path that is not a file.
fn file_parts(found: FileRead) -> Result<(Option<github::File>, CodeFileContent, Option<String>)> {
    match found {
        FileRead::Text { file, text } => Ok((Some(file), CodeFileContent::Text, Some(text))),
        FileRead::Binary(file) => Ok((Some(file), CodeFileContent::Binary, None)),
        FileRead::TooLarge(file) => Ok((file, CodeFileContent::TooLarge, None)),
        FileRead::Directory => Err(wrong_kind("That path is a directory, not a file.")),
        FileRead::Symlink { .. } => Err(wrong_kind(
            "That path is a link the hub does not follow, not a file.",
        )),
        FileRead::Submodule { .. } => Err(wrong_kind("That path is a submodule, not a file.")),
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "CodeQuery")]
impl Query {
    /// One directory. ref null: the default branch. path empty: the root. VALIDATION when the path is a file. Names only: open to a read key.
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.codeDirectory"
    )]
    async fn code_directory(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        #[graphql(name = "ref")] git_ref: Option<String>,
        #[graphql(default_with = "Some(String::new())")] path: Option<String>,
        #[graphql(default_with = "Some(false)")] fresh: Option<bool>,
    ) -> Result<CodeDirectoryView> {
        let target = forge::target(ctx, &repository).await?;
        let path = path.unwrap_or_default();
        let git_ref = git_ref.as_deref();
        let request = forge::checked(github::directory_request(&target.name, &path, git_ref))?;
        let keep = if git_ref.is_some_and(is_commit) {
            COMMIT_DIRECTORY
        } else {
            BRANCH_DIRECTORY
        };
        let reply = forge::read(ctx, &target, request, keep.fresh(fresh.unwrap_or(false)))
            .await?
            .ok_or_else(|| missing("path"))?;
        let DirectoryRead::Directory(found) = forge::parsed(&reply, github::directory_parse)?
        else {
            return Err(wrong_kind("That path is a file, not a directory."));
        };
        let place = Place {
            repository: target.repository,
            git_ref: shown(git_ref, &target),
            path: &path,
        };
        Ok(directory(found, &place, forge::stamp(&reply)))
    }

    /// One file, never kept by the hub. VALIDATION when the path is a directory. No MCP tool returns it (operator, 2026-11-14; ruling number to be assigned). Login or browser token only (D-23, R-100).
    #[graphql(guard = "Authenticated")]
    #[expect(
        clippy::too_many_arguments,
        reason = "K1 fixes the argument list of Query.codeFile"
    )]
    async fn code_file(
        &self,
        ctx: &Context<'_>,
        repository: ID,
        #[graphql(name = "ref")] git_ref: Option<String>,
        path: String,
    ) -> Result<CodeFileView> {
        operator_only(ctx, "codeFile")?;
        let target = forge::target(ctx, &repository).await?;
        if path.is_empty() {
            return Err(wrong_kind("Give the path of a file."));
        }
        let git_ref = git_ref.as_deref();
        let request = forge::checked(github::file_request(&target.name, &path, git_ref))?;
        let reply = forge::read(ctx, &target, request, Keep::never())
            .await?
            .ok_or_else(|| missing("path"))?;
        let found = file_read(&reply.response()).map_err(|_| forge::unusable())?;
        let place = Place {
            repository: target.repository,
            git_ref: shown(git_ref, &target),
            path: &path,
        };
        Ok(file(file_parts(found)?, &place, &target.home, reply.at()))
    }
}
