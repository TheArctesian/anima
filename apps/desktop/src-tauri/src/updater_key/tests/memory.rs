//! After startup the app is not dumpable, so no core dump of it holds the opened key (D-19).

use std::process::Command;

use crate::updater_key::keep_out_of_core_dumps;

/// Set in the copy of the test binary that makes itself not dumpable, so this test process
/// itself stays as it was.
const PROBE: &str = "ANIMA_TEST_DUMPABLE_PROBE";

#[test]
fn d_19_the_process_is_not_dumpable_after_startup() {
    if std::env::var_os(PROBE).is_some() {
        keep_out_of_core_dumps().unwrap();
        assert!(!nix::sys::prctl::get_dumpable().unwrap());
        return;
    }
    let path = module_path!().split_once("::").unwrap().1;
    let name = format!("{path}::d_19_the_process_is_not_dumpable_after_startup");
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &name, "--test-threads=1"])
        .env(PROBE, "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(
        stdout.contains("1 passed"),
        "the probe did not run: {stdout}"
    );
}
