//! A directory or a file of the repository (PRJ-033). What GitHub sends is untrusted text and
//! untrusted bytes: nothing here parses a file, and a symlink is described, never followed.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Deserialize;
use source_exchange::{Body, Problem, Request, Response};

use super::path::{contents_tail, ref_value};
use super::{get, read, unreadable};

const SUBJECT: &str = "code";

/// GitHub lists at most this many entries of one directory.
const LISTING_CAP: usize = 1_000;
/// A file over this many bytes is not read (GitHub's own limit for the default reply is 1 MB).
const MOST_FILE_BYTES: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Submodule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// One path segment. Text from GitHub.
    pub name: String,
    /// From the repository root. Text from GitHub.
    pub path: String,
    pub kind: EntryKind,
    /// Bytes; 0 for anything but a file.
    pub size: u64,
    pub sha: String,
}

/// A directory, directories first and then by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Directory {
    pub entries: Vec<Entry>,
    /// GitHub lists at most 1,000 entries: this one may have more.
    pub partial: bool,
}

/// What a directory request got.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectoryRead {
    Directory(Directory),
    /// The path names a file, a symlink or a submodule.
    NotADirectory,
}

/// A file, without its content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub sha: String,
}

/// What a file request got.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileRead {
    /// The content, decoded as UTF-8 and holding no NUL. Bytes from a repository: untrusted.
    Text { file: File, text: String },
    /// Not valid UTF-8, or it holds a NUL. No content is kept.
    Binary(File),
    /// Over the size this crate reads. The file is `None` when the reply was cut before it
    /// could be read.
    TooLarge(Option<File>),
    /// The path names a directory.
    Directory,
    /// A link to another path. The target is text, never followed.
    Symlink {
        name: String,
        path: String,
        target: String,
    },
    Submodule {
        name: String,
        path: String,
        /// The commit in the submodule repository.
        sha: String,
        git_url: Option<String>,
    },
}

#[derive(Deserialize)]
struct Raw {
    #[serde(rename = "type")]
    kind: String,
    name: String,
    path: String,
    size: Option<u64>,
    sha: String,
    content: Option<String>,
    encoding: Option<String>,
    target: Option<String>,
    submodule_git_url: Option<String>,
    download_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Answer {
    Directory(Vec<Raw>),
    One(Box<Raw>),
}

impl Raw {
    /// The kind of entry. On the 2022-11-28 version a submodule in a listing has the type
    /// `file`; it is told apart by its `submodule_git_url`, or by a null `download_url`, which
    /// every real file has. A type this crate does not know is `None`.
    fn entry_kind(&self) -> Option<EntryKind> {
        match self.kind.as_str() {
            "dir" => Some(EntryKind::Directory),
            "symlink" => Some(EntryKind::Symlink),
            "submodule" => Some(EntryKind::Submodule),
            "file" if self.submodule_git_url.is_some() || self.download_url.is_none() => {
                Some(EntryKind::Submodule)
            }
            "file" => Some(EntryKind::File),
            _ => None,
        }
    }

    fn file(&self) -> File {
        File {
            name: self.name.clone(),
            path: self.path.clone(),
            size: self.size.unwrap_or(0),
            sha: self.sha.clone(),
        }
    }
}

/// `GET /repos/{o}/{r}/contents/{path}`, with `ref` when one is given. The path is checked and
/// encoded segment by segment; the empty path is the repository root. The ref is a query value.
fn request(repository: &str, path: &str, git_ref: Option<&str>) -> Result<Request, Problem> {
    let tail = contents_tail(path)?;
    let mut filters = Vec::new();
    if let Some(git_ref) = git_ref {
        filters.push(("ref", ref_value(git_ref, SUBJECT)?));
    }
    get(repository, &tail, &filters, None)
}

/// The contents of a directory (the root when `path` is empty). A path or ref that cannot go
/// into a request, or a repository name that cannot, is refused before any request is built.
///
/// # Errors
/// A [`Problem`] when the repository name, the path or the ref cannot go into a request.
pub fn directory_request(
    repository: &str,
    path: &str,
    git_ref: Option<&str>,
) -> Result<Request, Problem> {
    request(repository, path, git_ref)
}

/// The same request for a file.
///
/// # Errors
/// A [`Problem`] when the repository name, the path or the ref cannot go into a request.
pub fn file_request(
    repository: &str,
    path: &str,
    git_ref: Option<&str>,
) -> Result<Request, Problem> {
    request(repository, path, git_ref)
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a directory or a file.
pub fn directory_parse(response: &Response<'_>) -> Result<DirectoryRead, Problem> {
    let Answer::Directory(raw) = read(response, SUBJECT)? else {
        return Ok(DirectoryRead::NotADirectory);
    };
    let partial = raw.len() >= LISTING_CAP;
    let mut entries: Vec<Entry> = raw
        .into_iter()
        .filter_map(|raw| {
            let kind = raw.entry_kind()?;
            Some(Entry {
                size: if kind == EntryKind::File {
                    raw.size.unwrap_or(0)
                } else {
                    0
                },
                name: raw.name,
                path: raw.path,
                kind,
                sha: raw.sha,
            })
        })
        .collect();
    entries.sort_by_cached_key(|entry| {
        (
            entry.kind != EntryKind::Directory,
            entry.name.to_lowercase(),
            entry.name.clone(),
        )
    });
    Ok(DirectoryRead::Directory(Directory { entries, partial }))
}

/// # Errors
/// A [`Problem`] when the status is not 200 or the body is not a file, a link or a directory.
pub fn file_parse(response: &Response<'_>) -> Result<FileRead, Problem> {
    if response.status == 200 && response.body == Body::TooLarge {
        return Ok(FileRead::TooLarge(None));
    }
    let Answer::One(raw) = read(response, SUBJECT)? else {
        return Ok(FileRead::Directory);
    };
    match raw.entry_kind() {
        Some(EntryKind::File) => file_content(&raw),
        Some(EntryKind::Directory) => Ok(FileRead::Directory),
        Some(EntryKind::Symlink) => Ok(FileRead::Symlink {
            target: raw.target.ok_or_else(|| unreadable(SUBJECT))?,
            name: raw.name,
            path: raw.path,
        }),
        Some(EntryKind::Submodule) => Ok(FileRead::Submodule {
            name: raw.name,
            path: raw.path,
            sha: raw.sha,
            git_url: raw.submodule_git_url,
        }),
        None => Err(unreadable(SUBJECT)),
    }
}

/// The decoded content of a file. Size is judged from the reply's `size` before decoding, and
/// again on the decoded bytes; no lossy decoding.
fn file_content(raw: &Raw) -> Result<FileRead, Problem> {
    let file = raw.file();
    let content = raw.content.as_deref().unwrap_or_default();
    if file.size > MOST_FILE_BYTES || (content.is_empty() && file.size > 0) {
        return Ok(FileRead::TooLarge(Some(file)));
    }
    if raw.encoding.as_deref().is_some_and(|name| name != "base64") {
        return Err(unreadable(SUBJECT));
    }
    // GitHub wraps the base64 in lines.
    let packed: Vec<u8> = content
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    let bytes = STANDARD.decode(packed).map_err(|_| unreadable(SUBJECT))?;
    if bytes.len() as u64 > MOST_FILE_BYTES {
        return Ok(FileRead::TooLarge(Some(file)));
    }
    match String::from_utf8(bytes) {
        Ok(text) if !text.contains('\0') => Ok(FileRead::Text { file, text }),
        _ => Ok(FileRead::Binary(file)),
    }
}
