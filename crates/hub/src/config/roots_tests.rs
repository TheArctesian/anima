//! The web folder must be absolute, not `/`, and free of `..` parts (D-2).

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
