//! The web folder, the data keys and the file pieces must be absolute, not `/`, and free of
//! `..` parts (D-2, VLT-003, FIL-001).

use super::tests::env;
use super::*;

#[test]
fn d_2_web_root_must_be_absolute_and_not_the_file_system_root() {
    for relative in ["web", "./web", " "] {
        let error = from_env(&env(&[(WEB_ROOT, relative)])).unwrap_err();
        assert!(
            matches!(error, Error::NotAbsolute { variable, .. } if variable == WEB_ROOT),
            "{relative:?}"
        );
    }
    let error = from_env(&env(&[(WEB_ROOT, "/")])).unwrap_err();
    assert!(matches!(error, Error::FilesystemRoot { variable } if variable == WEB_ROOT));
}

#[test]
fn d_2_roots_with_a_dot_dot_part_are_refused_naming_the_variable() {
    for bad in ["/..", "/usr/..", "/srv/../web"] {
        let error = from_env(&env(&[(WEB_ROOT, bad)])).unwrap_err();
        assert!(
            matches!(error, Error::ParentDirectory { variable } if variable == WEB_ROOT),
            "{bad:?}"
        );
    }
    let config = from_env(&env(&[(WEB_ROOT, "/srv/web")])).unwrap();
    assert_eq!(config.web_root, Some(PathBuf::from("/srv/web")));
}

#[test]
fn vlt_003_fil_001_the_data_key_and_file_folders_are_optional_absolute_roots() {
    let config = from_env(&env(&[])).unwrap();
    assert_eq!((config.data_key_dir, config.files_dir), (None, None));

    let config = from_env(&env(&[
        (DATA_KEY_DIR, "/srv/hub/data-keys"),
        (FILES_DIR, "/srv/hub/files"),
    ]))
    .unwrap();
    assert_eq!(
        config.data_key_dir,
        Some(PathBuf::from("/srv/hub/data-keys"))
    );
    assert_eq!(config.files_dir, Some(PathBuf::from("/srv/hub/files")));

    for variable in [DATA_KEY_DIR, FILES_DIR] {
        let error = from_env(&env(&[(variable, "files")])).unwrap_err();
        assert!(matches!(error, Error::NotAbsolute { variable: named, .. } if named == variable));
        let error = from_env(&env(&[(variable, "/")])).unwrap_err();
        assert!(matches!(error, Error::FilesystemRoot { variable: named } if named == variable));
    }
}
