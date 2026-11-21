//! The rights of the services of a recipe (SF-224): each service refuses new rights
//! and drops each capability, no setting makes a service root, and no setting makes an image
//! change its code at the start. The rules read the file as its author wrote it, before an
//! environment file fills its variables, so that no value of the install changes the result.

use compose_rules::Node;

use crate::entry::Exception;

/// The one value of `security_opt` that each service has.
const NO_NEW_PRIVILEGES: &str = "no-new-privileges:true";
/// The `cap_drop` of each service with no exception.
const ALL: &str = "ALL";
/// Each capability that the engine gives by default other than the five of
/// [`Exception::InitCapabilities`], in the order of the names. Contract K10 has no `cap_add`:
/// a service with that exception drops these, and keeps only the five, or fewer on an engine
/// with a smaller default set.
const OTHER_DEFAULTS: [&str; 9] = [
    "AUDIT_WRITE",
    "FSETID",
    "KILL",
    "MKNOD",
    "NET_BIND_SERVICE",
    "NET_RAW",
    "SETFCAP",
    "SETPCAP",
    "SYS_CHROOT",
];

/// Why the rights of a recipe are refused. The code is fixed and holds no text of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Privilege {
    /// `DOCKER_MODS`: the image gets and runs more code at the start.
    ImageMods,
    /// `INSTALL_PACKAGES`: the image installs packages at the start.
    Packages,
    /// `INSTALL_PIP_PACKAGES`: the image installs Python packages at the start.
    PipPackages,
    /// `VERSION` is not `docker`: the image gets a different version at the start.
    VersionNotFixed,
    /// `PUID` or `PGID` is not a whole number above 0.
    RootIdentity,
    /// `user` is root (`0` or `root`, as the user or as the group), or a variable.
    RootUser,
    /// A service has no `security_opt: [no-new-privileges:true]`.
    NewPrivileges,
    /// `cap_drop` is not `[ALL]`, and not the drop of an exception that the entry names.
    CapabilitiesKept,
    /// The entry names an exception that no service uses.
    ExceptionNotNeeded,
}

impl Privilege {
    /// The code as the report writes it.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ImageMods => "image_mods",
            Self::Packages => "install_packages",
            Self::PipPackages => "install_pip_packages",
            Self::VersionNotFixed => "version_not_fixed",
            Self::RootIdentity => "root_identity",
            Self::RootUser => "root_user",
            Self::NewPrivileges => "new_privileges",
            Self::CapabilitiesKept => "capabilities_kept",
            Self::ExceptionNotNeeded => "exception_not_needed",
        }
    }
}

/// The scalars of a key that is one scalar or a list of them.
fn texts<'a>(service: &'a Node, key: &str) -> Vec<&'a str> {
    match service.get(key) {
        Some(Node::Text(text)) => vec![text],
        Some(Node::List(items)) => items.iter().filter_map(Node::text).collect(),
        _ => Vec::new(),
    }
}

/// The name and the value of each variable of `environment`: a map, or a list of `NAME=value`.
pub(crate) fn environment(service: &Node) -> Vec<(&str, &str)> {
    match service.get("environment") {
        Some(Node::Map(entries)) => entries
            .iter()
            .filter_map(|(name, value)| Some((name.as_str(), value.text()?)))
            .collect(),
        Some(Node::List(items)) => items
            .iter()
            .filter_map(|item| item.text()?.split_once('='))
            .collect(),
        _ => Vec::new(),
    }
}

/// A user or a group identifier that is not root: a whole number above 0, written in the file.
fn is_not_root_number(text: &str) -> bool {
    text.bytes().all(|byte| byte.is_ascii_digit()) && text.parse::<u32>().is_ok_and(|id| id > 0)
}

/// `user` as `name`, `id`, `name:group` or `id:group`, with no part that is root or empty, and
/// no variable.
fn is_not_root_user(text: &str) -> bool {
    !text.contains('$')
        && text.split(':').all(|part| {
            !part.is_empty()
                && part != "root"
                && (is_not_root_number(part) || !part.bytes().all(|byte| byte.is_ascii_digit()))
        })
}

/// The settings of one service that make it root or change its code at the start.
fn settings(service: &Node) -> Vec<Privilege> {
    let mut found: Vec<Privilege> = environment(service)
        .into_iter()
        .filter_map(|(name, value)| match name {
            "DOCKER_MODS" => Some(Privilege::ImageMods),
            "INSTALL_PACKAGES" => Some(Privilege::Packages),
            "INSTALL_PIP_PACKAGES" => Some(Privilege::PipPackages),
            "VERSION" if value != "docker" => Some(Privilege::VersionNotFixed),
            "PUID" | "PGID" if !is_not_root_number(value) => Some(Privilege::RootIdentity),
            _ => None,
        })
        .collect();
    if !texts(service, "user").into_iter().all(is_not_root_user) {
        found.push(Privilege::RootUser);
    }
    found
}

/// `Some(true)` when the service drops the capabilities of the exception, `Some(false)` when it
/// drops each one, and `None` when it keeps more than that.
fn dropped(service: &Node, init: bool) -> Option<bool> {
    let mut drops = texts(service, "cap_drop");
    drops.sort_unstable();
    if drops == [ALL] {
        Some(false)
    } else if init && drops == OTHER_DEFAULTS {
        Some(true)
    } else {
        None
    }
}

/// The codes of the rights of `services`, for an entry that names `exceptions`. Each code is
/// there one time, in a fixed order.
pub(crate) fn check(services: &[(String, Node)], exceptions: &[Exception]) -> Vec<Privilege> {
    let init = exceptions.contains(&Exception::InitCapabilities);
    let mut found = Vec::new();
    let mut used = false;
    for (_, service) in services {
        found.extend(settings(service));
        if !texts(service, "security_opt").contains(&NO_NEW_PRIVILEGES) {
            found.push(Privilege::NewPrivileges);
        }
        match dropped(service, init) {
            Some(exception) => used |= exception,
            None => found.push(Privilege::CapabilitiesKept),
        }
    }
    if init && !used {
        found.push(Privilege::ExceptionNotNeeded);
    }
    found.sort_unstable();
    found.dedup();
    found
}
