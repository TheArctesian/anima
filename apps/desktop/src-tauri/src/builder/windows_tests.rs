//! Native build processes, file boundaries and the Windows release plan (WIN2).
use super::{
    artifact,
    child::Running,
    environment::Environment,
    error::BuilderError,
    job::{Target, Version},
    plan::{self, Program},
};
use std::{path::Path, sync::Arc, time::Duration};

#[test]
#[ignore = "requires installed Microsoft C++ Build Tools and a Windows SDK"]
fn start_menu_environment_builds_with_the_discovered_windows_compiler() {
    // Remove developer-shell paths and variables: discovery must work from a normal app.
    let inherited = std::env::vars_os().filter(|(name, _)| {
        ["SystemRoot", "WINDIR", "USERPROFILE", "TEMP", "TMP"]
            .iter()
            .any(|keep| name.eq_ignore_ascii_case(keep))
    });
    let environment = Environment::from_parent(inherited).with_compiler();
    let compiler = environment
        .find("cl", Path::is_file)
        .expect("installed MSVC");
    assert!(environment.find("rc", Path::is_file).is_some());
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(
        folder.path().join("probe.c"),
        "#include <windows.h>\nint main(void) { return GetCurrentProcessId() == 0; }\n",
    )
    .unwrap();
    let built = std::process::Command::new(compiler)
        .args(["/nologo", "/Fe:probe.exe", "probe.c"])
        .current_dir(folder.path())
        .env_clear()
        .envs(environment.for_step(&[]))
        .output()
        .unwrap();
    assert!(built.status.success(), "{built:?}");
    assert!(
        std::process::Command::new(folder.path().join("probe.exe"))
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn windows_builder_needs_native_test_runtimes() {
    use super::tools::{self, Tool};
    let env = Environment::from_parent([("PATH".into(), r"C:\tools".into())]);
    for (program, tool) in [("gcc.exe", Tool::GoRaceCompiler), ("node.exe", Tool::Node)] {
        let missing = tools::missing(Target::WindowsX86_64, &env, |path| {
            path.file_name().is_some_and(|name| name != program)
        });
        assert_eq!(missing, [tool]);
    }
    assert_eq!(tools::missing(Target::WindowsX86_64, &env, |_| true), []);
}

#[test]
fn windows_search_is_case_insensitive_and_never_passes_secrets() {
    let env = Environment::from_parent([
        ("Path".into(), r"C:\tools;.;relative".into()),
        ("USERPROFILE".into(), r"C:\Users\Ada".into()),
        ("SystemRoot".into(), r"C:\Windows".into()),
        ("ANIMA_ACCESS_TOKEN".into(), "fixture-secret".into()),
    ]);
    assert_eq!(
        env.find("cargo", |path| path == Path::new(r"C:\tools\cargo.exe")),
        Some(r"C:\tools\cargo.exe".into())
    );
    let passed = env.for_step(&[]);
    assert!(
        passed
            .iter()
            .any(|(name, value)| name == "GIT_CONFIG_GLOBAL" && value == "NUL")
    );
    assert!(passed.iter().all(|(_, value)| value != "fixture-secret"));
    assert!(env.dirs().iter().all(|path| path.is_absolute()));
}

#[test]
fn windows_artifacts_use_the_nsis_installer_for_updates() {
    let version = Version::parse("0.2.0").unwrap();
    let clone = Path::new(r"C:\build\anima");
    assert_eq!(Target::here(), Ok(Target::WindowsX86_64));
    assert!(
        artifact::updater_file(clone, Target::WindowsX86_64, &version)
            .ends_with("nsis/Anima_0.2.0_x64-setup.exe")
    );
    let steps = plan::bundle(clone, Target::WindowsX86_64, &version, None);
    assert_eq!(steps.len(), 2);
    assert!(steps[1].args.iter().any(|arg| arg == "nsis"));
}

#[test]
fn an_upload_refuses_a_hard_link_to_another_file() {
    let folder = tempfile::tempdir().unwrap();
    anima_windows::files::protect(folder.path()).unwrap();
    let original = folder.path().join("original");
    std::fs::write(&original, b"fixture").unwrap();
    let clone = folder.path().join("clone");
    anima_windows::files::directory(&clone).unwrap();
    let alias = clone.join("bundle.exe");
    std::fs::hard_link(&original, &alias).unwrap();
    assert!(artifact::open_own(&clone, &alias).is_err());
    std::fs::remove_file(&alias).unwrap();
    std::fs::write(&alias, b"own bundle").unwrap();
    assert!(artifact::open_own(&clone, &alias).is_ok());
}

#[tokio::test]
async fn cancellation_stops_a_native_build_and_releases_its_slot() {
    let running = Running::default();
    let claim = running.claim(Arc::new(|_| {})).unwrap();
    assert!(matches!(
        running.claim(Arc::new(|_| {})),
        Err(BuilderError::Busy)
    ));
    let root = std::env::var_os("SystemRoot").unwrap();
    let program = Path::new(&root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let command = plan::command(
        Program::Git,
        &std::env::temp_dir(),
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 60",
        ],
    );
    let env = Environment::from_parent(std::env::vars_os());
    let cancel = async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        running.cancel();
    };
    let work = claim.run(&command, &program, env.for_step(&[]));
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(work, cancel).0
    })
    .await
    .unwrap();
    assert_eq!(result, Err(BuilderError::Cancelled));
    drop(claim);
    assert!(running.claim(Arc::new(|_| {})).is_ok());
}
