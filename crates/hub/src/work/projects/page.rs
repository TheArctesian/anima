//! Keyset paging of projects by name, then id (API-006). Pure.

use std::fmt::Write as _;

use crate::{api_errors::FieldError, ids::ProjectId};

const CURSOR_PREFIX: &str = "p1.";
const MAX_PAGE_SIZE: usize = 200;

/// The place a cursor points after: the project's name in lower case and its id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct After {
    pub name: String,
    pub id: ProjectId,
}

/// The page size the operator asked for (`first`), 1 to 200.
///
/// # Errors
/// A `first` error when it is outside that range.
pub fn page_size(first: i32) -> Result<usize, FieldError> {
    match usize::try_from(first) {
        Ok(size) if (1..=MAX_PAGE_SIZE).contains(&size) => Ok(size),
        _ => Err(FieldError::new(
            "first",
            format!("Ask for between 1 and {MAX_PAGE_SIZE} projects per page."),
        )),
    }
}

/// The opaque cursor just after `after`.
#[must_use]
pub fn encode_cursor(after: &After) -> String {
    let mut cursor = String::from(CURSOR_PREFIX);
    for byte in after.name.bytes() {
        write!(cursor, "{byte:02x}").expect("writing to a String cannot fail");
    }
    write!(cursor, ".{}", after.id).expect("writing to a String cannot fail");
    cursor
}

/// The place a cursor of [`encode_cursor`] points after.
///
/// # Errors
/// An `after` error when the text is not such a cursor.
pub fn decode_cursor(cursor: &str) -> Result<After, FieldError> {
    let bad = || {
        FieldError::new(
            "after",
            "That page cursor is not valid. Start again from the first page.",
        )
    };
    let rest = cursor.strip_prefix(CURSOR_PREFIX).ok_or_else(bad)?;
    let (hex, id) = rest.split_once('.').ok_or_else(bad)?;
    if !hex.is_ascii() || hex.len() % 2 != 0 {
        return Err(bad());
    }
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|_| bad())?;
    Ok(After {
        name: String::from_utf8(bytes).map_err(|_| bad())?,
        id: id.parse().map_err(|_| bad())?,
    })
}
