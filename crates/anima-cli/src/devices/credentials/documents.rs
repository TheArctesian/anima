//! The credential documents. The variables of a `*Set` document hold the password or key, so
//! they derive no `Debug` (the generated code adds `Serialize` only, to send them) and the
//! caller wipes them after the request (CMD-014, CMD-025, LOG-007). The answers say only
//! whether a credential is kept, so they may print.

use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/credentials/set_ssh.graphql",
    response_derives = "Debug, Serialize",
    skip_serializing_none
)]
pub struct SshCredentialSet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/credentials/set_rdp.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct RdpCredentialSet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/credentials/set_vnc.graphql",
    response_derives = "Debug, Serialize"
)]
pub struct VncCredentialSet;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/credentials/clear_ssh.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct SshCredentialClear;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/credentials/clear_rdp.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct RdpCredentialClear;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "../../contracts/schema.graphql",
    query_path = "src/devices/credentials/clear_vnc.graphql",
    response_derives = "Debug, Serialize",
    variables_derives = "Debug"
)]
pub struct VncCredentialClear;

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::marker::PhantomData;

    use super::*;

    /// `Shows::<T>::DEBUG` is true only when `T: Debug`.
    struct Shows<T>(PhantomData<T>);
    trait Hidden {
        const DEBUG: bool = false;
    }
    impl<T> Hidden for Shows<T> {}
    impl<T: Debug> Shows<T> {
        const DEBUG: bool = true;
    }

    #[test]
    fn cmd_014_the_variables_that_hold_a_secret_cannot_be_printed_with_debug() {
        assert!(!Shows::<<SshCredentialSet as GraphQLQuery>::Variables>::DEBUG);
        assert!(!Shows::<<RdpCredentialSet as GraphQLQuery>::Variables>::DEBUG);
        assert!(!Shows::<<VncCredentialSet as GraphQLQuery>::Variables>::DEBUG);
        assert!(!Shows::<super::ssh_credential_set::SshCredentialInput>::DEBUG);
        assert!(Shows::<<SshCredentialClear as GraphQLQuery>::Variables>::DEBUG);
    }
}
