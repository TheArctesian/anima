//! Making the operator's GitHub App from a manifest: the name, the manifest, the pages the
//! operator presses on and the readers for GitHub's replies (PRJ-039, SRC-006). Pure.

mod converted;
mod installation;
mod manifest;
mod paths;

use source_exchange::Registration;

pub(crate) const REGISTRATION: Registration = Registration {
    web_base: "https://github.com",
    name: manifest::name,
    manifest: manifest::manifest,
    form_path: paths::form_path,
    install_path: paths::install_path,
    settings_path: paths::settings_path,
    conversion_path: paths::conversion_path,
    parse_converted: converted::parse_converted,
    installation_path: paths::installation_path,
    installations_path: "/app/installations",
    parse_installation: installation::parse_installation,
    parse_installations: installation::parse_installations,
};

#[cfg(test)]
mod tests;
