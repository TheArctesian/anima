//! Pages of a hub connection and `--all` joining them into one (CMD-026). The one pager: the
//! hub's answers are not trusted, so a cursor that comes back or a list that never ends stops
//! the command with a failure instead of running forever (S4).

use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;

use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::shell_word;

/// The hub's largest page for devices, sessions, events, audit entries and deliveries.
pub const MAX_PAGE: i64 = 200;

/// The most rows `--all` collects before it stops.
pub const MAX_ROWS: usize = 100_000;

/// A page of rows and where the next one starts.
pub trait Page {
    /// What the hub calls the place a page starts after: a cursor string, a line sequence.
    type Cursor: Clone + Eq + Hash + Display;
    /// The cursor of the next page, when there is one.
    fn next_cursor(&self) -> Option<Self::Cursor>;
    /// Adds the rows of `later`, which becomes the last page.
    fn append(&mut self, later: Self);
    /// Says there is no next page: `--all` has read them all.
    fn end(&mut self);
    /// How many rows this holds.
    fn edge_count(&self) -> usize;
}

/// Implements [`Page`] for a generated K1 connection with `edges` and `page_info`.
#[macro_export]
macro_rules! connection_page {
    ($connection:ty) => {
        impl $crate::paging::Page for $connection {
            type Cursor = String;

            fn next_cursor(&self) -> Option<String> {
                self.page_info
                    .end_cursor
                    .clone()
                    .filter(|_| self.page_info.has_next_page)
            }

            fn append(&mut self, later: Self) {
                self.edges.extend(later.edges);
                self.page_info = later.page_info;
            }

            fn end(&mut self) {
                self.page_info.has_next_page = false;
            }

            fn edge_count(&self) -> usize {
                self.edges.len()
            }
        }
    };
}

/// The page after `after`, or with `all` every page joined into one. `fetch` takes the references
/// it needs by copy and returns an `async move` block, so the future owns what it uses.
///
/// # Errors
/// The failure of `fetch`; [`Failure::Paging`] when the hub repeats a cursor or `--all` passes
/// [`MAX_ROWS`].
pub async fn fetch<P: Page, Next: Future<Output = Result<P, Failure>>>(
    after: Option<P::Cursor>,
    all: bool,
    mut fetch: impl FnMut(Option<P::Cursor>) -> Next,
) -> Result<P, Failure> {
    let mut asked: HashSet<P::Cursor> = after.iter().cloned().collect();
    let mut joined = fetch(after).await?;
    if !all {
        return Ok(joined);
    }
    while let Some(cursor) = joined.next_cursor() {
        if joined.edge_count() > MAX_ROWS {
            return Err(Failure::Paging {
                reason: format!(
                    "more than {MAX_ROWS} rows; narrow the filter or page with --after"
                ),
            });
        }
        if !asked.insert(cursor.clone()) {
            return Err(Failure::Paging {
                reason: format!(
                    "the hub sent the cursor {cursor} again after {} rows; it is not paging",
                    joined.edge_count()
                ),
            });
        }
        joined.append(fetch(Some(cursor)).await?);
    }
    joined.end();
    Ok(joined)
}

/// The page size: `--limit`, or the largest page for `--all`.
#[must_use]
pub const fn page_size(limit: i64, all: bool, largest: i64) -> i64 {
    if all { largest } else { limit }
}

/// A command as the operator would type it, for a hint that repeats the filters in use.
#[derive(Clone, Debug)]
pub struct CommandLine(Vec<String>);

impl CommandLine {
    #[must_use]
    pub fn new(command: &str) -> Self {
        Self(vec![command.to_owned()])
    }

    /// Adds `word` as it is typed, quoted when it needs it.
    #[must_use]
    pub fn word(mut self, word: &str) -> Self {
        self.0.push(shell_word(word));
        self
    }

    /// Adds `--name value` when there is a value.
    #[must_use]
    pub fn flag(self, name: &str, value: Option<&str>) -> Self {
        self.flags(name, value)
    }

    /// Adds `--name value` for each value.
    #[must_use]
    pub fn flags<'a>(mut self, name: &str, values: impl IntoIterator<Item = &'a str>) -> Self {
        for value in values {
            self.0.push(format!("{name} {}", shell_word(value)));
        }
        self
    }

    /// Adds `--name choice` for each choice of a `clap::ValueEnum`.
    #[must_use]
    pub fn choices<'a, T: clap::ValueEnum + 'a>(
        self,
        name: &str,
        choices: impl IntoIterator<Item = &'a T>,
    ) -> Self {
        let names: Vec<String> = choices
            .into_iter()
            .filter_map(clap::ValueEnum::to_possible_value)
            .map(|value| value.get_name().to_owned())
            .collect();
        self.flags(name, names.iter().map(String::as_str))
    }

    /// Adds `--name` when `on`.
    #[must_use]
    pub fn switch(mut self, name: &str, on: bool) -> Self {
        if on {
            self.0.push(name.to_owned());
        }
        self
    }
}

impl Display for CommandLine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0.join(" "))
    }
}

/// Tells a person where the next page starts (not with `--json`).
pub fn note_next(invocation: &Invocation, page: &impl Page, what: &str, command: &CommandLine) {
    if let Some(cursor) = page.next_cursor() {
        let cursor = shell_word(&cursor.to_string());
        invocation.output.note(&format!(
            "{} {what}. Next page: {command} --after {cursor} (or --all)",
            page.edge_count()
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::future::ready;

    use super::*;

    struct Fake {
        edges: Vec<u32>,
        next: Option<String>,
    }

    impl Page for Fake {
        type Cursor = String;

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
        fn edge_count(&self) -> usize {
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
        let asked = RefCell::new(Vec::new());
        let joined = fetch(None, true, |after: Option<String>| {
            asked.borrow_mut().push(after.clone());
            ready(Ok(match after.as_deref() {
                None => page(&[1, 2], Some("a")),
                _ => page(&[3], None),
            }))
        })
        .await
        .expect("pages");
        assert_eq!((joined.edges, joined.next), (vec![1, 2, 3], None));
        assert_eq!(*asked.borrow(), vec![None, Some("a".to_owned())]);

        let first = fetch(Some("x".to_owned()), false, |_| {
            ready(Ok(page(&[1], Some("y"))))
        })
        .await
        .expect("page");
        assert_eq!(first.next.as_deref(), Some("y"));
    }

    #[tokio::test]
    async fn cmd_026_s4_a_cursor_that_comes_back_stops_the_pager() {
        let calls = Cell::new(0);
        let stopped = fetch(None, true, |_| {
            calls.set(calls.get() + 1);
            ready(Ok(page(&[1], Some("same"))))
        })
        .await;
        assert!(matches!(stopped, Err(Failure::Paging { .. })));
        assert_eq!(
            calls.get(),
            2,
            "asked for the cursor once, then saw it again"
        );

        let echoed = fetch(Some("c".to_owned()), true, |_| {
            ready(Ok(page(&[1], Some("c"))))
        })
        .await;
        assert!(
            matches!(echoed, Err(Failure::Paging { .. })),
            "the cursor it was given"
        );
    }

    #[tokio::test]
    async fn cmd_026_s4_all_stops_after_the_row_cap() {
        let counter = Cell::new(0_u32);
        let stopped = fetch(None, true, |_| {
            counter.set(counter.get() + 1);
            ready(Ok(page(&[0; 1000], Some(&format!("c{}", counter.get())))))
        })
        .await;
        let Err(Failure::Paging { reason }) = stopped else {
            panic!("expected the pager to stop");
        };
        assert!(reason.contains("100000"), "{reason}");
        assert!(
            counter.get() <= 102,
            "stopped near the cap: {}",
            counter.get()
        );
    }

    #[test]
    fn cmd_026_a_next_page_command_keeps_each_filter_quoted() {
        let command = CommandLine::new("anima x list")
            .flag("--name", Some("web server"))
            .flag("--site", None)
            .flags("--tag", ["a", "b"])
            .switch("--live", true)
            .switch("--other", false);
        assert_eq!(
            command.to_string(),
            "anima x list --name 'web server' --tag a --tag b --live"
        );
    }
}
