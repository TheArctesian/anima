//! The web folder, the data keys and the file pieces must be absolute, not `/`, and free of
//! `..` parts (D-2, VLT-003, FIL-001).

use super::tests::{env, host_path};
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
    let error = from_env(&env(&[(WEB_ROOT, &host_path("/"))])).unwrap_err();
    assert!(matches!(error, Error::FilesystemRoot { variable } if variable == WEB_ROOT));
}

#[test]
fn d_2_roots_with_a_dot_dot_part_are_refused_naming_the_variable() {
    for bad in [
        &host_path("/.."),
        &host_path("/usr/.."),
        &host_path("/srv/../web"),
    ] {
        let error = from_env(&env(&[(WEB_ROOT, bad)])).unwrap_err();
        assert!(
            matches!(error, Error::ParentDirectory { variable } if variable == WEB_ROOT),
            "{bad:?}"
        );
    }
    let config = from_env(&env(&[(WEB_ROOT, &host_path("/srv/web"))])).unwrap();
    assert_eq!(config.web_root, Some(PathBuf::from(host_path("/srv/web"))));
}

#[test]
fn vlt_003_fil_001_the_data_key_and_file_folders_are_optional_absolute_roots() {
    let config = from_env(&env(&[])).unwrap();
    assert_eq!((config.data_key_dir, config.files_dir), (None, None));

    let config = from_env(&env(&[
        (DATA_KEY_DIR, &host_path("/srv/hub/data-keys")),
        (FILES_DIR, &host_path("/srv/hub/files")),
    ]))
    .unwrap();
    assert_eq!(
        config.data_key_dir,
        Some(PathBuf::from(host_path("/srv/hub/data-keys")))
    );
    assert_eq!(
        config.files_dir,
        Some(PathBuf::from(host_path("/srv/hub/files")))
    );

    for variable in [DATA_KEY_DIR, FILES_DIR] {
        let error = from_env(&env(&[(variable, "files")])).unwrap_err();
        assert!(matches!(error, Error::NotAbsolute { variable: named, .. } if named == variable));
        let error = from_env(&env(&[(variable, &host_path("/"))])).unwrap_err();
        assert!(matches!(error, Error::FilesystemRoot { variable: named } if named == variable));
    }
}
