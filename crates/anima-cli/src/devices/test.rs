//! `anima device test`: can the hub reach a saved device and sign in? The answer carries the
//! host key or certificate the hub saw, so the operator can compare it before trusting it.

use super::documents::{DeviceTest, device_test};
use super::get::resolve;
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::{Cell, Table, Tone};

#[derive(Debug, clap::Args)]
pub struct TestArgs {
    /// The device's ID or its exact name.
    device: String,
    /// Which access to test.
    #[arg(long, value_enum, default_value_t = ProtocolArg::Ssh)]
    protocol: ProtocolArg,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ProtocolArg {
    Ssh,
    Rdp,
    Vnc,
}

impl TestArgs {
    /// Prints what the hub found.
    ///
    /// # Errors
    /// [`Failure::NotFound`] for an unknown device; the hub's refusal. A device that cannot be
    /// reached is a result, not a failure.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let input = device_test::ConnectionTestInput {
            device_id: Some(resolve(&hub, &self.device).await?),
            host: None,
            ssh: None,
            credential: None,
            protocol: Some(self.protocol.k1()),
            rdp: None,
            vnc: None,
            password: None,
        };
        let result = hub
            .run::<DeviceTest>(device_test::Variables { input })
            .await?
            .test_connection;
        invocation.output.data(&result, table)?;
        if let Some(key) = &result.host_key {
            invocation.output.note(&format!(
                "To trust this host key: anima device trust accept-host-key {} {}",
                self.device, key.fingerprint_sha256
            ));
        }
        if let Some(certificate) = result
            .server_certificate
            .as_ref()
            .filter(|_| !result.certificate_matched)
        {
            invocation.output.note(&format!(
                "To trust this certificate: anima device trust accept-certificate {} {}",
                self.device, certificate.fingerprint_sha256
            ));
        }
        Ok(())
    }
}

fn table(result: &device_test::DeviceTestTestConnection) -> Table {
    let mut table = Table::record();
    table.pair("Reachable", yes_no(result.reachable, true));
    table.pair("Signed in", yes_no(result.authenticated, true));
    if let Some(key) = &result.host_key {
        table.pair(
            "Host key",
            format!("{} {}", key.algorithm, key.fingerprint_sha256),
        );
        table.pair("Host key changed", yes_no(result.host_key_changed, false));
    }
    if let Some(certificate) = &result.server_certificate {
        table.pair("Certificate", certificate.fingerprint_sha256.as_str());
        table.pair(
            "Certificate subject",
            certificate.subject.as_deref().unwrap_or("-"),
        );
        table.pair(
            "Certificate changed",
            yes_no(result.certificate_changed, false),
        );
    }
    table.pair(
        "Certificate matches pin",
        yes_no(result.certificate_matched, true),
    );
    if let Some(message) = &result.message {
        table.pair("Message", message.as_str());
    }
    table
}

/// "yes" or "no", green when it is the wanted answer and red when it is not.
fn yes_no(value: bool, wanted: bool) -> Cell {
    let tone = if value == wanted {
        Tone::Good
    } else {
        Tone::Bad
    };
    Cell::toned(if value { "yes" } else { "no" }, tone)
}

impl ProtocolArg {
    const fn k1(self) -> device_test::Protocol {
        match self {
            Self::Ssh => device_test::Protocol::SSH,
            Self::Rdp => device_test::Protocol::RDP,
            Self::Vnc => device_test::Protocol::VNC,
        }
    }
}
