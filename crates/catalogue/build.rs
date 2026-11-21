//! Builds the recipes of `entries/` into the crate (task H38): one `include_str!` for each file
//! of each recipe folder, so that the hub carries the catalogue with no file on the disk. The
//! static test runs on these texts when the hub reads them; nothing is checked here.

use std::{env, fmt::Write as _, fs, path::Path};

/// The three files of a recipe folder (Section 13.1 of the proposal).
const FILES: [&str; 3] = ["entry.json", "compose.yaml", "environment.example"];

fn main() {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("cargo sets it")).join("entries");
    println!("cargo::rerun-if-changed={}", root.display());
    let mut folders: Vec<_> = fs::read_dir(&root)
        .expect("the entries folder is readable")
        .map(|found| found.expect("the entries folder is readable").path())
        .filter(|path| path.is_dir())
        .collect();
    folders.sort();
    let mut code = String::from("&[\n");
    for folder in folders {
        let name = folder
            .file_name()
            .expect("a folder has a name")
            .to_string_lossy();
        write!(code, "    Folder {{ name: {name:?}, texts: [").expect("a String takes text");
        for file in FILES {
            let path = folder.join(file);
            if path.is_file() {
                let path = path.display().to_string();
                write!(code, "Some(include_str!({path:?})), ")
            } else {
                write!(code, "None, ")
            }
            .expect("a String takes text");
        }
        code.push_str("] },\n");
    }
    code.push(']');
    let out = Path::new(&env::var("OUT_DIR").expect("cargo sets it")).join("shipped.rs");
    fs::write(out, code).expect("the build folder is writable");
}
