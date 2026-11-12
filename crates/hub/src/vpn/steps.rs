//! The commands that join a Linux computer to the VPN with a one-use key (vpn.md Section 5,
//! VPN-001, VPN-003). Pure. The operator types the key at a hidden prompt into a file that only
//! root can read, so the key is never in a command line, a shell history or a step.

use ipnet::IpNet;

/// The Headscale server the computers log in to.
const LOGIN_SERVER: &str = "https://hs.anima.ing";
/// Step 1: the key, read without echo into a root-only file.
const READ_KEY: &str = r#"sudo bash -c 'umask 077; read -rsp "Pre-auth key: " K; echo; printf "%s" "$K" > /run/anima-preauth'"#;
/// Step 3: the key file goes as soon as the computer is on the VPN.
const REMOVE_KEY: &str = "sudo rm -f /run/anima-preauth";

/// The steps for the subnet router of the policy site `site` (`[a-z0-9-]{1,32}`, checked by
/// the caller) that serves `subnet`. No `--advertise-tags`: the key carries the tags.
pub(super) fn router(site: &str, subnet: IpNet) -> Vec<String> {
    steps(&format!(
        "--advertise-routes={} --hostname {site}-router",
        subnet.trunc()
    ))
}

/// The steps for a Linux computer of the operator: it uses the site routes.
pub(super) fn computer() -> Vec<String> {
    steps("--accept-routes")
}

fn steps(flags: &str) -> Vec<String> {
    vec![
        READ_KEY.to_owned(),
        format!(
            "sudo tailscale up --login-server {LOGIN_SERVER} --auth-key=file:/run/anima-preauth {flags}"
        ),
        REMOVE_KEY.to_owned(),
    ]
}
