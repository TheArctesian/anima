//! Secret-file fixtures use the host's access controls.
use std::path::Path;
pub fn mode(path: &Path, mode: u32) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
    #[cfg(windows)]
    {
        anima_windows::files::protect(path).unwrap();
        if mode & 0o077 != 0 {
            let root = std::env::var_os("SystemRoot").unwrap();
            let status = std::process::Command::new(Path::new(&root).join("System32/icacls.exe"))
                .arg(path)
                .args(["/grant", "*S-1-1-0:R"])
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap();
            assert!(status.success());
        }
    }
}
pub fn assert_private(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    #[cfg(windows)]
    assert!(anima_windows::files::secure(path, true).unwrap());
}

/// The refusal names the access controls of the current host.
pub fn assert_refused(stderr: &str) {
    #[cfg(unix)]
    assert!(
        stderr.contains("0600") || stderr.contains("chmod 600"),
        "{stderr}"
    );
    #[cfg(windows)]
    assert!(stderr.contains("permission denied"), "{stderr}");
}
