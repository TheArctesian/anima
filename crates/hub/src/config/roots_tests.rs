//! The web and recording folders must be absolute, not `/`, and free of `..` parts (D-2).

use super::tests::env;
use super::*;

#[test]
fn d_2_web_root_and_recording_root_must_be_absolute_and_not_the_file_system_root() {
    for variable in [WEB_ROOT, RECORDING_ROOT] {
        for relative in ["web", "./web", " "] {
            let error = from_env(&env(&[(variable, relative)])).unwrap_err();
            assert!(
                matches!(error, Error::NotAbsolute { variable: named, .. } if named == variable),
                "{variable} {relative:?}"
            );
        }
        let error = from_env(&env(&[(variable, "/")])).unwrap_err();
        assert!(matches!(error, Error::FilesystemRoot { variable: named } if named == variable));
    }
}

#[test]
fn d_2_roots_with_a_dot_dot_part_are_refused_naming_the_variable() {
    for variable in [WEB_ROOT, RECORDING_ROOT] {
        for bad in ["/..", "/usr/..", "/srv/../web"] {
            let error = from_env(&env(&[(variable, bad)])).unwrap_err();
            assert!(
                matches!(error, Error::ParentDirectory { variable: named } if named == variable),
                "{variable} {bad:?}"
            );
        }
        let config = from_env(&env(&[(variable, "/srv/web")])).unwrap();
        assert!(config.web_root.or(config.recording_root).is_some());
    }
}
