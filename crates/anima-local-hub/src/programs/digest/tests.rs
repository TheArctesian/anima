//! The order of the entries that the hash step gets does not change the digest.

use std::fs;

use super::{Entry, Kind, Walk};
use crate::programs::{MOST_BYTES, MOST_ENTRIES};

fn entries(folder: &std::path::Path, names: &[&str]) -> Walk {
    let mut walk = Walk {
        entries: Vec::new(),
        budget: crate::programs::Budget {
            entries: MOST_ENTRIES,
            bytes: MOST_BYTES,
        },
    };
    for name in names {
        fs::write(folder.join(name), name).unwrap();
        walk.entries.push(Entry {
            name: name.into(),
            from: folder.join(name),
            kind: Kind::File { program: false },
        });
    }
    walk
}

#[test]
fn the_same_entries_in_two_orders_give_one_digest() {
    let folder = tempfile::tempdir().unwrap();
    let names = ["b", "a", "c", "ab", "a b", "z"];
    let mut reversed = names;
    reversed.reverse();

    let one = entries(folder.path(), &names).hash().unwrap();
    let two = entries(folder.path(), &reversed).hash().unwrap();

    assert_eq!(one, two);
}
