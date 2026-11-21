use std::path::PathBuf;

use crate::model_work::cause::Cause;
use crate::model_work::command::{environment, llama_arguments};
use crate::model_work::folders::{Installation, layout};
use crate::model_work::machine::{MachineSettings, SettingsError};
use crate::model_work::names::{Backend, Runtime};
use crate::model_work::releases::{ArchiveFormat, Release, SourceEntry, accepted};
use crate::model_work::settings::ServerSettings;
use crate::wire::WorkType;

#[test]
fn the_machine_settings_have_the_defaults_of_the_operator() {
    let settings = MachineSettings::parse("{}").expect("reads");
    assert!(!settings.sandbox && !settings.second_hash);
    assert_eq!(settings.revision_age_days, 0);
    assert_eq!(settings.organisations, ["unsloth", "mlx-community"]);
    assert_eq!(settings.agent_types.len(), 10);
    let strict = MachineSettings::parse(
        r#"{"sandbox":true,"agentTypes":["model_report"],"organisations":["unsloth"]}"#,
    )
    .expect("reads");
    assert!(strict.sandbox);
    assert_eq!(strict.agent_types, [WorkType::ModelReport]);
    assert_eq!(
        MachineSettings::parse(r#"{"agentTypes":["build"]}"#),
        Err(SettingsError::Value("agentTypes"))
    );
    assert!(matches!(
        MachineSettings::parse(r#"{"switch":true}"#),
        Err(SettingsError::Unknown(_))
    ));
}

#[test]
fn the_lists_of_the_program_refuse_old_releases() {
    let release = |version: &str| Release {
        runtime: Runtime::LlamaServer,
        version: version.to_owned(),
        backend: Backend::Metal,
    };
    assert_eq!(accepted(&release("b8100")), Ok(()));
    assert_eq!(accepted(&release("b100")), Err(Cause::RuntimeTooOld));
    assert_eq!(Release::number(Runtime::LlamaServer, "8100"), None);
    assert_eq!(Release::number(Runtime::LlamaServer, "b81a"), None);
}

#[test]
fn the_command_line_binds_loopback_and_holds_no_key() {
    let words = llama_arguments(&ServerSettings::default(), 41000);
    assert_eq!(&words[..4], ["--host", "127.0.0.1", "--port", "41000"]);
    assert!(words.iter().all(|word| !word.contains("key")));
    let env = environment(&PathBuf::from("/s/tmp"), &PathBuf::from("/r/bin"), "k");
    let names: Vec<&str> = env.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "HOME",
            "TMPDIR",
            "PATH",
            "HF_HUB_OFFLINE",
            "TRANSFORMERS_OFFLINE",
            "VLLM_NO_USAGE_STATS",
            "DO_NOT_TRACK",
            "LLAMA_API_KEY"
        ]
    );
}

#[test]
fn the_folders_follow_the_table() {
    let env = |name: &str| match name {
        "HOME" => Some("/home/u".to_owned()),
        _ => None,
    };
    let linux = layout(Installation::LinuxUser, env).expect("folders");
    assert_eq!(
        linux.runtimes,
        PathBuf::from("/home/u/.local/share/anima-daemon/runtimes")
    );
    assert_eq!(
        linux.state,
        PathBuf::from("/home/u/.local/state/anima-daemon/model-state")
    );
    let mac = layout(Installation::MacUser, env).expect("folders");
    assert_eq!(
        mac.models,
        PathBuf::from("/home/u/Library/Application Support/anima-daemon/models")
    );
    let system = layout(Installation::LinuxSystem, env).expect("folders");
    assert_eq!(
        system.runtimes,
        PathBuf::from("/var/lib/anima-daemon/runtimes")
    );
    let set = |name: &str| (name == "ANIMA_DAEMON_HOME").then(|| "/d".to_owned());
    assert_eq!(
        layout(Installation::MacUser, set).expect("folders").state,
        PathBuf::from("/d/model-state")
    );
    assert_eq!(layout(Installation::LinuxUser, |_| None), None);
}

#[test]
fn the_form_of_an_archive_comes_from_the_end_of_its_address() {
    let form = |address: &str| {
        SourceEntry {
            runtime: "llama.cpp".into(),
            version: "b6000".into(),
            os: "linux".into(),
            architecture: "x86_64".into(),
            backend: "cpu".into(),
            address: address.into(),
            sha256: String::new(),
            size_bytes: 1,
            program: "llama-server".into(),
        }
        .format()
    };
    assert_eq!(form("https://h/a/r.tar.gz"), Some(ArchiveFormat::TarGz));
    assert_eq!(form("https://h/a/r.TGZ?x=1"), Some(ArchiveFormat::TarGz));
    assert_eq!(form("https://h/a/r.Zip#f"), Some(ArchiveFormat::Zip));
    assert_eq!(form("https://h/a/r.gz"), None);
    assert_eq!(form("https://h/a/r.tar"), None);
}

#[test]
fn each_failed_step_of_containment_has_its_own_detail_and_launcher_code() {
    use crate::model_work::cause::Containment;
    let steps = [
        Containment::NoDelegation,
        Containment::NoNsdelegate,
        Containment::CgroupJoinFailed,
        Containment::BackstopFailed,
        Containment::ParentGone,
        Containment::UsernsDenied,
        Containment::ExecFailed,
    ];
    for step in steps {
        let cause = Cause::ContainmentFailed(step);
        assert_eq!(
            (cause.as_str(), cause.detail()),
            ("containment_failed", Some(step.as_str()))
        );
        if let Some(code) = step.exit_code() {
            assert_eq!(Containment::of_exit_code(code), Some(step));
            assert_ne!(code, 125);
        }
    }
    assert_eq!(Containment::UsernsDenied.as_str(), "userns_denied");
    assert_eq!(Containment::of_exit_code(125), None);
    assert_eq!(Cause::Listener.detail(), None);
}
