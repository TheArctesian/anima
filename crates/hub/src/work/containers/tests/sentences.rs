//! Each error of a read as the operator gets it: the K7 code and a fixed sentence that says
//! what to do (API-010, HOST-001).

use async_graphql::Value;

use crate::{
    fleet::ssh::LinkError,
    work::containers::{Error, graphql::to_graphql},
};

/// The code and the message of `error` on the wire.
fn stated(error: Error) -> (String, String) {
    let error = to_graphql(error);
    let code = error.extensions.as_ref().and_then(|e| e.get("code"));
    let Some(Value::String(code)) = code else {
        panic!("no code: {error:?}");
    };
    (code.clone(), error.message)
}

#[test]
fn host_001_a_device_with_no_ssh_credential_is_conflict_and_the_answer_says_so() {
    assert_eq!(
        stated(Error::Link(LinkError::NoCredential)),
        (
            "CONFLICT".to_owned(),
            "Docker was not read: the device has no SSH credential in Anima. Anima reads the \
             containers of a device through SSH."
                .to_owned()
        )
    );
}

#[test]
fn k7_what_the_operator_can_fix_is_conflict_and_a_silent_device_is_unavailable() {
    let code = |error| stated(error).0;
    for fixable in [
        Error::NoDocker,
        Error::Link(LinkError::SshOff),
        Error::Link(LinkError::NoHostKey),
        Error::Link(LinkError::HostKeyChanged),
        Error::Link(LinkError::CredentialRefused),
        Error::Link(LinkError::RefusedTarget),
    ] {
        assert_eq!(code(fixable), "CONFLICT");
    }
    for silent in [
        Error::DockerFailed,
        Error::Unreadable,
        Error::Link(LinkError::Unreachable),
        Error::Link(LinkError::TimeLimit),
        Error::Link(LinkError::Lost),
    ] {
        assert_eq!(code(silent), "UNAVAILABLE");
    }
}

#[test]
fn k7_an_unknown_device_or_container_is_not_found_and_a_refused_value_is_validation() {
    let device = "0190a000-0000-7000-8002-0000000000ff".parse().unwrap();
    assert_eq!(stated(Error::NotFound(device)).0, "NOT_FOUND");
    assert_eq!(
        stated(Error::NoContainer),
        (
            "NOT_FOUND".to_owned(),
            "The device has no container with this identifier.".to_owned()
        )
    );
    assert_eq!(stated(Error::InvalidContainerId).0, "VALIDATION");
    assert_eq!(stated(Error::InvalidLineCount).0, "VALIDATION");
}
