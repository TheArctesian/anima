//! Pages of a K1 connection without `totalCount` (events, audit, deliveries), and `--all`
//! joining them into one (CMD-026).

use crate::failure::Failure;
use crate::invocation::Invocation;

/// The hub's largest page; `--all` asks for pages of this size.
pub const MAX_PAGE: i64 = 200;

/// A generated connection: its edges and where the next page starts.
pub trait Page {
    /// The cursor of the next page, when there is one.
    fn next_cursor(&self) -> Option<String>;
    /// Adds the edges of `later`, which becomes the last page.
    fn append(&mut self, later: Self);
    /// Says there is no next page: `--all` has read them all.
    fn end(&mut self);
    /// How many edges this holds.
    fn len(&self) -> usize;
}

/// The page after `after`, or with `all` every page joined into one.
///
/// # Errors
/// The failure of `fetch`.
pub async fn fetch<P: Page>(
    after: Option<String>,
    all: bool,
    mut fetch: impl AsyncFnMut(Option<String>) -> Result<P, Failure>,
) -> Result<P, Failure> {
    let mut joined = fetch(after).await?;
    while all {
        let Some(cursor) = joined.next_cursor() else {
            break;
        };
        joined.append(fetch(Some(cursor)).await?);
    }
    if all {
        joined.end();
    }
    Ok(joined)
}

/// The page size: `--limit`, or the largest page for `--all`.
#[must_use]
pub const fn page_size(limit: i64, all: bool) -> i64 {
    if all { MAX_PAGE } else { limit }
}

/// Tells a person where the next page starts (not with `--json`).
pub fn note_next(invocation: &Invocation, page: &impl Page, what: &str, command: &str) {
    if let Some(cursor) = page.next_cursor() {
        invocation.output.note(&format!(
            "{} {what}. Next page: {command} --after {cursor} (or --all)",
            page.len()
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        edges: Vec<u32>,
        next: Option<String>,
    }

    impl Page for Fake {
        fn next_cursor(&self) -> Option<String> {
            self.next.clone()
        }
        fn append(&mut self, later: Self) {
            self.edges.extend(later.edges);
            self.next = later.next;
        }
        fn end(&mut self) {
            self.next = None;
        }
        fn len(&self) -> usize {
            self.edges.len()
        }
    }

    fn page(edges: &[u32], next: Option<&str>) -> Fake {
        Fake {
            edges: edges.to_vec(),
            next: next.map(str::to_owned),
        }
    }

    #[tokio::test]
    async fn cmd_026_all_joins_every_page_and_one_page_stops_at_the_first() {
        let mut asked = Vec::new();
        let joined = fetch(None, true, async |after: Option<String>| {
            asked.push(after.clone());
            Ok(match after.as_deref() {
                None => page(&[1, 2], Some("a")),
                _ => page(&[3], None),
            })
        })
        .await
        .expect("pages");
        assert_eq!((joined.edges, joined.next), (vec![1, 2, 3], None));
        assert_eq!(asked, vec![None, Some("a".to_owned())]);

        let first = fetch(Some("x".to_owned()), false, async |_| {
            Ok(page(&[1], Some("y")))
        })
        .await
        .expect("page");
        assert_eq!(first.next.as_deref(), Some("y"));
    }
}
