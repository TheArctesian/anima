//! The K1 shapes of a directory and a file of a repository, from the provider's types. Pure.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::{Enum, ID, SimpleObject};
use chrono::{DateTime, Utc};
use provider_github::live::contents::{Directory, Entry, EntryKind, File};

use super::address;
use crate::{
    ids::RepositoryId,
    work::projects::forge::{ForgeRead, int},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum CodeEntryKind {
    File,
    Directory,
    Symlink,
    Submodule,
}

#[derive(SimpleObject)]
#[graphql(name = "CodeEntry")]
pub struct CodeEntryView {
    /// One path segment. Text from GitHub.
    name: String,
    /// From the repository root, no leading slash. Text from GitHub.
    path: String,
    kind: CodeEntryKind,
    /// Bytes; 0 for anything but a FILE.
    size: i32,
    sha: String,
}

fn entry(entry: Entry) -> CodeEntryView {
    CodeEntryView {
        name: entry.name,
        path: entry.path,
        kind: match entry.kind {
            EntryKind::File => CodeEntryKind::File,
            EntryKind::Directory => CodeEntryKind::Directory,
            EntryKind::Symlink => CodeEntryKind::Symlink,
            EntryKind::Submodule => CodeEntryKind::Submodule,
        },
        size: int(entry.size),
        sha: entry.sha,
    }
}

#[derive(SimpleObject)]
#[graphql(name = "CodeDirectory")]
pub struct CodeDirectoryView {
    repository: ID,
    /// The branch name or commit asked for.
    #[graphql(name = "ref")]
    git_ref: String,
    /// Empty for the root.
    path: String,
    /// Directories first, then by name.
    entries: Vec<CodeEntryView>,
    /// True: GitHub lists at most 1,000 entries of one directory and this one may have more.
    partial: bool,
    read: ForgeRead,
}

/// What a directory or file read was asked: the repository, the ref as K1 shows it and the
/// path from the root.
pub struct Place<'a> {
    pub repository: RepositoryId,
    pub git_ref: &'a str,
    pub path: &'a str,
}

/// The K1 directory of `found` at `place`, read as `read` says.
#[must_use]
pub fn directory(found: Directory, place: &Place<'_>, read: ForgeRead) -> CodeDirectoryView {
    CodeDirectoryView {
        repository: ID(place.repository.to_string()),
        git_ref: place.git_ref.to_owned(),
        path: place.path.to_owned(),
        entries: found.entries.into_iter().map(entry).collect(),
        partial: found.partial,
        read,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub enum CodeFileContent {
    /// text holds the file.
    Text,
    /// Not valid UTF-8, or it holds a NUL byte. No content is returned.
    Binary,
    /// Larger than 1,000,000 bytes. No content is returned.
    TooLarge,
}

/// Bytes from a repository: untrusted. The hub does not look for secrets in it and makes no promise that it holds none.
#[derive(SimpleObject)]
#[graphql(name = "CodeFile")]
pub struct CodeFileView {
    repository: ID,
    #[graphql(name = "ref")]
    git_ref: String,
    /// The path GitHub read. When the path asked is a link to a file in the same repository, the path of that file.
    path: String,
    name: String,
    sha: String,
    /// Bytes.
    size: i32,
    content: CodeFileContent,
    /// Only for TEXT. Plain text: never render it as HTML and never run it.
    text: Option<String>,
    /// The file on GitHub (PRJ-010).
    url: String,
    read_at: DateTime<Utc>,
}

/// The K1 file `file` asked at `place`, whose repository page is `home`, read at `read_at`.
/// GitHub answers a link to a file in the same repository with that file, so the path shown
/// is the one GitHub read (decisions item 13). `None` for the file: the reply was too large to
/// say which file it was; the place asked stands in, with no sha and a size of 0.
#[must_use]
pub fn file(
    found: (Option<File>, CodeFileContent, Option<String>),
    place: &Place<'_>,
    home: &str,
    read_at: DateTime<Utc>,
) -> CodeFileView {
    let (file, content, text) = found;
    let file = file.unwrap_or_else(|| File {
        name: place.path.rsplit('/').next().unwrap_or_default().to_owned(),
        path: place.path.to_owned(),
        size: 0,
        sha: String::new(),
    });
    CodeFileView {
        repository: ID(place.repository.to_string()),
        git_ref: place.git_ref.to_owned(),
        url: address::file(home, place.git_ref, &file.path),
        path: file.path,
        name: file.name,
        sha: file.sha,
        size: int(file.size),
        content,
        text,
        read_at,
    }
}
