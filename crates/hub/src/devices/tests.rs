mod host;
mod page;
mod rules;

use super::*;
use crate::ids::SiteId;

const HOME: &str = "0190a000-0000-7000-8001-000000000001";

fn home() -> SiteId {
    HOME.parse().unwrap()
}

fn new_device(name: &str, host: &str) -> NewDeviceInput {
    NewDeviceInput {
        site: home(),
        name: name.into(),
        host: host.into(),
        kind: "Proxmox VE node".into(),
        os: OperatingSystem::Linux,
        tags: vec![],
        ssh: None,
    }
}

fn ssh(enabled: bool, port: Option<i32>, username: &str) -> SshInput {
    SshInput {
        enabled,
        port,
        username: username.into(),
        auth: SshAuth::PrivateKey,
    }
}

fn fields(errors: &[FieldError]) -> Vec<&'static str> {
    errors.iter().map(|error| error.field).collect()
}

fn host_error(host: &str) -> Option<FieldError> {
    validate_new_device(&new_device("pve-01", host))
        .err()
        .and_then(|mut e| e.pop())
}

#[test]
fn dev_010_enums_round_trip_through_database_text() {
    for os in [
        OperatingSystem::Linux,
        OperatingSystem::Macos,
        OperatingSystem::Windows,
        OperatingSystem::Other,
    ] {
        assert_eq!(os.as_str().parse::<OperatingSystem>().unwrap(), os);
    }
    for auth in [SshAuth::Password, SshAuth::PrivateKey] {
        assert_eq!(auth.as_str().parse::<SshAuth>().unwrap(), auth);
    }
    assert_eq!(SshAuth::PrivateKey.as_str(), "private_key");
    assert!("beos".parse::<OperatingSystem>().is_err());
    assert!("Linux".parse::<OperatingSystem>().is_err());
    assert!("".parse::<SshAuth>().is_err());
}

#[test]
fn dev_010_graphql_enum_names_follow_the_schema() {
    use async_graphql::EnumType;
    let os: Vec<_> = OperatingSystem::items()
        .iter()
        .map(|item| item.name)
        .collect();
    assert_eq!(os, ["LINUX", "MACOS", "WINDOWS", "OTHER"]);
    let auth: Vec<_> = SshAuth::items().iter().map(|item| item.name).collect();
    assert_eq!(auth, ["PASSWORD", "PRIVATE_KEY"]);
}

#[test]
fn error_names_what_was_not_found() {
    let error = Error::NotFound(Missing::Site(home()));
    assert_eq!(error.to_string(), format!("site {HOME} not found"));
}
