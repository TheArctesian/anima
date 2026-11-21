//! Reads one repository into the variants and the facts that the library keeps (LIBM-002 to
//! LIBM-005). The source is approved before the first read, so a repository outside the list
//! is never fetched.

use std::collections::HashMap;

use sha2::{Digest, Sha256};

use super::{
    Error,
    fetch::{Fetch, FetchError, LONGEST_FILE},
    model::{Facts, Refusal, Variant, VariantFile},
    naming::{self, Repository},
    repository::{self, Listing, ModelConfig},
    variants::{self, Planned},
};

/// What the hub read, ready to store.
#[derive(Debug)]
pub struct Read {
    pub name: String,
    pub facts: Facts,
    pub variants: Vec<Variant>,
}

pub(super) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, each| {
        let _ = write!(text, "{each:02x}");
        text
    })
}

fn is_revision(text: &str) -> bool {
    text.len() == 40
        && text
            .bytes()
            .all(|each| matches!(each, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|each| matches!(each, b'0'..=b'9' | b'a'..=b'f'))
}

/// Whether a path from the answer is safe to put in a URL path: no empty, `.` or `..` part.
fn is_plain_path(path: &str) -> bool {
    path.split('/')
        .all(|part| !part.is_empty() && part != "." && part != "..")
}

async fn config(
    fetch: &impl Fetch,
    repository: &str,
    listing: &Listing,
) -> Result<Option<ModelConfig>, Error> {
    if !listing.has_config() {
        return Ok(None);
    }
    let text = fetch.file(repository, &listing.sha, "config.json").await?;
    repository::model_config(&text)
        .map(Some)
        .map_err(|_| Error::Source(FetchError::Unavailable))
}

/// Each file of the variants with its SHA-256 value; a file outside LFS is read and hashed
/// once, however many variants list it.
async fn hashed(
    fetch: &impl Fetch,
    repository: &str,
    revision: &str,
    planned: Vec<Planned>,
) -> Result<Vec<Variant>, Error> {
    let mut computed: HashMap<String, String> = HashMap::new();
    let mut found = Vec::with_capacity(planned.len());
    for variant in planned {
        let mut files = Vec::with_capacity(variant.files.len());
        for file in &variant.files {
            let sha256 = match &file.sha256 {
                Some(known) if is_sha256(known) => known.clone(),
                Some(_) => return Err(Error::Refused(Refusal::NoHash)),
                None => {
                    if file.size_bytes > LONGEST_FILE || !is_plain_path(&file.path) {
                        return Err(Error::Refused(Refusal::NoHash));
                    }
                    if !computed.contains_key(&file.path) {
                        let bytes = fetch.file(repository, revision, &file.path).await?;
                        computed.insert(file.path.clone(), hex(&Sha256::digest(&bytes)));
                    }
                    computed[&file.path].clone()
                }
            };
            files.push(VariantFile {
                path: file.path.clone(),
                size_bytes: file.size_bytes,
                sha256,
            });
        }
        found.push(Variant {
            tag: variant.tag(),
            format: variant.format,
            quantisation: variant.quantisation.clone(),
            repository: repository.to_owned(),
            revision: revision.to_owned(),
            files,
        });
    }
    Ok(found)
}

/// Reads the repository for the model `name`.
///
/// # Errors
/// `Refused` when the organisation is not approved (before any read), when the repository is
/// closed or has no variant, or when a file has no SHA-256 value; `Source` when the source
/// fails or answers in a form the hub does not read.
pub async fn read(
    fetch: &impl Fetch,
    source: &Repository,
    name: String,
    approved: &[String],
) -> Result<Read, Error> {
    if !naming::is_approved(&source.organisation, approved) {
        return Err(Error::Refused(Refusal::NotApproved));
    }
    let full = source.full();
    let listing = repository::listing(&fetch.repository(&full).await?)
        .map_err(|_| Error::Source(FetchError::Unavailable))?;
    if !listing.id.eq_ignore_ascii_case(&full) {
        return Err(Error::Source(FetchError::Unavailable));
    }
    if !listing.is_open() {
        return Err(Error::Refused(Refusal::Closed));
    }
    if !is_revision(&listing.sha) {
        return Err(Error::Source(FetchError::Unavailable));
    }
    let config = config(fetch, &full, &listing).await?;
    let planned = variants::plan(&listing, config.as_ref());
    if planned.is_empty() {
        return Err(Error::Refused(Refusal::NoVariant));
    }
    let variants = hashed(fetch, &full, &listing.sha, planned).await?;
    if variants
        .iter()
        .any(|variant| !naming::is_name(&variant.tag))
    {
        return Err(Error::Refused(Refusal::NoVariant));
    }
    // The publisher is the organisation of the base model on the card. The source is in the
    // approved list, so a repository outside the list never names one.
    let publisher = listing
        .base_organisation()
        .filter(|organisation| naming::organisation("publisher", organisation).is_ok());
    Ok(Read {
        name,
        facts: listing.facts(config.as_ref(), publisher),
        variants,
    })
}
