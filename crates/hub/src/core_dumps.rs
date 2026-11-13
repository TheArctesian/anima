//! Keeps the hub's memory out of core dumps (WP-36 D-45, R-116, SEC-004): the updater key's seed,
//! the vault key and the R2 token live in it. Compose also sets the core size limit to 0; this
//! covers a hub started any other way.

/// Marks the process not dumpable, so a crash writes no core file and other processes of the
/// same user cannot attach to it. Linux only; elsewhere it does nothing.
///
/// # Errors
///
/// The OS error when the kernel refuses.
pub fn forbid() -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    nix::sys::prctl::set_dumpable(false)?;
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::process::Command;

    /// Set in the child that checks itself, so the test process stays dumpable.
    const PROBE: &str = "ANIMA_CORE_DUMPS_PROBE";
    const MARK: &str = "core dumps forbidden";

    /// The child turns dumping off and checks; the parent checks that the child ran this test.
    #[test]
    fn d_45_the_hub_process_is_not_dumpable() {
        if std::env::var_os(PROBE).is_some() {
            assert!(nix::sys::prctl::get_dumpable().unwrap());
            super::forbid().unwrap();
            assert!(!nix::sys::prctl::get_dumpable().unwrap());
            println!("{MARK}");
            return;
        }
        let name = "core_dumps::tests::d_45_the_hub_process_is_not_dumpable";
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--nocapture", "--test-threads=1"])
            .env(PROBE, "1")
            .output()
            .unwrap();
        let out = String::from_utf8_lossy(&child.stdout);
        assert!(child.status.success(), "{out}");
        assert!(out.contains(MARK), "{out}");
    }
}
