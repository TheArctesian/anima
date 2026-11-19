//! Each error of a read as its K7 code with a fixed sentence (API-010).

use async_graphql::Error as GraphQlError;

use crate::{
    api_errors::{ErrorCode, FieldError, coded, internal, invalid},
    fleet::ssh::LinkError,
    work::containers::Error,
};

/// The GraphQL error with its `extensions.code` (K7) and the fixed sentence; a store failure
/// is `INTERNAL` with no detail.
pub(in crate::work::containers) fn to_graphql(error: Error) -> GraphQlError {
    match &error {
        Error::InvalidContainerId => invalid(&[FieldError::new(
            "containerId",
            "A container identifier is 12 to 64 lower-case hexadecimal characters.",
        )]),
        Error::InvalidLineCount => invalid(&[FieldError::new(
            "lines",
            "The number of lines is 1 to 1000.",
        )]),
        other => match stated(other) {
            Some((code, sentence)) => coded(sentence, code),
            None => internal(error),
        },
    }
}

/// The code and the sentence the operator reads; `None` for a store failure.
const fn stated(error: &Error) -> Option<(ErrorCode, &'static str)> {
    match error {
        Error::NotFound(_) => Some((ErrorCode::NotFound, "No device has this ID.")),
        Error::NoContainer => Some((
            ErrorCode::NotFound,
            "The device has no container with this identifier.",
        )),
        Error::NoDocker => Some((
            ErrorCode::Conflict,
            "Docker was not read: the device's SSH user finds no docker command. Anima \
             reads Docker only.",
        )),
        Error::DockerFailed => Some((
            ErrorCode::Unavailable,
            "Docker was not read: docker did not succeed on the device. Check that \
             Docker runs and that the device's SSH user may use it.",
        )),
        Error::Unreadable => Some((
            ErrorCode::Unavailable,
            "Docker was not read: the hub could not read the answer of docker.",
        )),
        Error::Link(link) => link_stated(link),
        Error::InvalidContainerId | Error::InvalidLineCount | Error::Devices(_) => None,
    }
}

/// [`stated`] for a link to the device that failed. What the operator can fix is `CONFLICT`;
/// a device that does not answer is `UNAVAILABLE`.
const fn link_stated(error: &LinkError) -> Option<(ErrorCode, &'static str)> {
    match error {
        LinkError::NoCredential => Some((
            ErrorCode::Conflict,
            "Docker was not read: the device has no SSH credential in Anima. Anima \
             reads the containers of a device through SSH.",
        )),
        LinkError::SshOff => Some((
            ErrorCode::Conflict,
            "Docker was not read: SSH is turned off for the device. Turn it on in the \
             device's SSH settings.",
        )),
        LinkError::NoHostKey => Some((
            ErrorCode::Conflict,
            "Docker was not read: no host key is accepted for the device. Test its \
             connection and accept its key.",
        )),
        LinkError::HostKeyChanged => Some((
            ErrorCode::Conflict,
            "Docker was not read: the device presented another host key than the \
             accepted one. No credential was sent.",
        )),
        LinkError::RefusedTarget => Some((
            ErrorCode::Conflict,
            "Docker was not read: the device's host resolves to an address the hub may \
             not connect to.",
        )),
        LinkError::CredentialForOtherHost
        | LinkError::CredentialRefused
        | LinkError::UnreadableKey => Some((
            ErrorCode::Conflict,
            "Docker was not read: the device did not accept the stored SSH \
                 credential. Enter it again in the device's SSH settings.",
        )),
        LinkError::Unresolved
        | LinkError::Unreachable
        | LinkError::Timeout
        | LinkError::NoKeyExchange
        | LinkError::Lost => Some((
            ErrorCode::Unavailable,
            "Docker was not read: the device cannot be reached over SSH.",
        )),
        LinkError::CommandRefused | LinkError::TimeLimit => Some((
            ErrorCode::Unavailable,
            "Docker was not read: the device did not finish the docker command in time.",
        )),
        LinkError::Trust(_) | LinkError::Credentials(_) => None,
    }
}
