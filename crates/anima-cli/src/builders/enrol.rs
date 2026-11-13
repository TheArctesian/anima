//! `anima builder enrol`: records this computer's VPN address as a builder and shows the builder
//! token, the one secret this command exists to show (BLD-023). The token goes to standard
//! output once, or with `--to-file` to a `0600` file; the answer derives neither `Debug` nor
//! `Serialize`.

use std::path::PathBuf;

use serde_json::json;
use zeroize::Zeroizing;

use super::documents::{BuilderEnrol, builder_enrol};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::vpn::key::{Secret, show_once};

#[derive(Debug, clap::Args)]
pub struct EnrolArgs {
    /// The builder's name.
    #[arg(long)]
    name: String,
    /// The operating system the builder runs.
    #[arg(long, value_enum)]
    os: OsArg,
    /// The processor architecture.
    #[arg(long, value_enum)]
    arch: ArchArg,
    /// Write the token to this file (mode 0600) instead of standard output.
    #[arg(long, value_name = "PATH")]
    to_file: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum OsArg {
    Linux,
    Macos,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ArchArg {
    #[value(name = "x86-64")]
    X8664,
    Aarch64,
}

impl EnrolArgs {
    /// Enrols the builder (from this request's address) and shows its token once.
    ///
    /// # Errors
    /// [`Failure`] for a hub refusal (a name in use, not a VPN address) or an unwritable file.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let variables = builder_enrol::Variables {
            name: self.name,
            platform: builder_enrol::BuilderPlatformInput {
                os: self.os.k1(),
                arch: self.arch.k1(),
            },
        };
        let answer = hub.run::<BuilderEnrol>(variables).await?.enrol_builder;
        let token = Zeroizing::new(answer.token);
        let builder = &answer.builder;
        invocation.output.note(&format!(
            "Builder {} ({} {}) enrolled from {}; id {}. The token expires {}.",
            builder.name,
            enum_name(&builder.platform.os),
            enum_name(&builder.platform.arch),
            builder.client_addr,
            builder.id,
            answer.expires_at
        ));
        let public = json!({
            "builder": {
                "id": builder.id,
                "name": builder.name,
                "platform": {
                    "os": enum_name(&builder.platform.os),
                    "arch": enum_name(&builder.platform.arch),
                },
                "clientAddr": builder.client_addr,
                "enrolledAt": builder.enrolled_at,
            },
            "expiresAt": answer.expires_at,
        });
        let secret = Secret {
            label: "builder token",
            field: "token",
            value: &token,
            to_file: self.to_file.as_deref(),
        };
        show_once(invocation, &secret, public)
    }
}

impl OsArg {
    const fn k1(self) -> builder_enrol::BuilderOs {
        match self {
            Self::Linux => builder_enrol::BuilderOs::LINUX,
            Self::Macos => builder_enrol::BuilderOs::MACOS,
        }
    }
}

impl ArchArg {
    const fn k1(self) -> builder_enrol::BuilderArch {
        match self {
            Self::X8664 => builder_enrol::BuilderArch::X86_64,
            Self::Aarch64 => builder_enrol::BuilderArch::AARCH64,
        }
    }
}
