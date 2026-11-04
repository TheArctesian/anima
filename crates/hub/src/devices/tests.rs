/// Wraps each async test in a throwaway database holding the K6 rows from
/// `tests/fixtures/devices.sql`. These tests live in the crate because they call the
/// store's `pub(super)` functions; public behaviour is tested in `tests/it`.
macro_rules! store_test {
    ($(async fn $name:ident($pool:ident: PgPool) $body:block)+) => {$(
        #[sqlx::test(
            migrations = "./migrations",
            fixtures(path = "../../../tests/fixtures", scripts("devices"))
        )]
        async fn $name($pool: sqlx::PgPool) $body
    )+};
}

mod host;
mod page;
mod rules;
mod store_read;
mod store_write;

use super::model::*;
use super::page::*;
use super::rules::*;
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

/// The last field error for a device whose only possibly bad field is `host`.
fn host_error(host: &str) -> Option<FieldError> {
    validate_new_device(&new_device("pve-01", host))
        .err()
        // Safe: name, kind, tags and ssh are valid here, so `host` is the only error to pop.
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
fn k7_not_found_error_names_the_missing_entity() {
    let error = Error::NotFound(Missing::Site(home()));
    assert_eq!(error.to_string(), format!("site {HOME} not found"));
}

#[test]
fn k7_validation_error_names_fields_never_values() {
    let error = Error::Validation(vec![
        FieldError::new("name", "The name must be 1 to 64 characters."),
        FieldError::new("ssh.port", "secret-ish 70000"),
    ]);
    assert_eq!(error.to_string(), "input is not valid: name, ssh.port");
}

const PVE_01: &str = "0190a000-0000-7000-8002-000000000001";
const JELLYFIN: &str = "0190a000-0000-7000-8002-000000000006";
const OFFICE_1: &str = "0190a000-0000-7000-8001-000000000002";
const OFFICE_2: &str = "0190a000-0000-7000-8001-000000000003";

fn instant(text: &str) -> chrono::DateTime<chrono::Utc> {
    text.parse().unwrap()
}

fn device_id(text: &str) -> crate::ids::DeviceId {
    text.parse().unwrap()
}

fn site_id(text: &str) -> SiteId {
    text.parse().unwrap()
}

/// A valid device record for the home site.
fn record(name: &str) -> NewDevice {
    NewDevice {
        site: home(),
        name: name.into(),
        host: format!("{name}.home.lan"),
        kind: "Test box".into(),
        os: OperatingSystem::Linux,
        tags: vec![],
        ssh: SshSettings::default(),
    }
}
