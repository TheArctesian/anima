//! The checks at the start of the program (contract K10 Section 6.4.1), as one decision over
//! facts that the caller reads from the operating system and from the engine. Nothing here
//! reads a file, a socket or a version.

use crate::{EngineKind, Settings};

/// The lowest version of the engine that the program starts with. From this version a port
/// that is published on `127.0.0.1` is closed to a different host of the local network.
pub const LOWEST_ENGINE: Version = Version(28, 0, 0);
/// The lowest version of the Compose program that the program starts with: the first that
/// reads each key of Section 6.4.4, also the `content` of a config.
pub const LOWEST_COMPOSE: Version = Version(2, 23, 1);

/// A version of three numbers. A version that the caller cannot read is never given: the
/// caller then refuses to start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u64, pub u64, pub u64);

/// One link, one real file or one folder of a path, as the operating system gives it without
/// following a link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The identifier of the owner.
    pub owner: u32,
    /// The mode bits.
    pub mode: u32,
    /// Each user of the group of the entry, also each user whose first group it is.
    pub group_members: Vec<u32>,
    /// Each user to whom the access list gives the right to write, directly or by a group.
    pub listed_writers: Vec<u32>,
    /// True when the access list gives the right to write to each user.
    pub listed_for_all: bool,
}

/// The facts of the start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartFacts {
    /// The identifier of the user that the program runs as.
    pub running_as: u32,
    /// Each administrator account: an account that can become the identifier 0 (on a Mac, a
    /// member of the group `admin`).
    pub administrators: Vec<u32>,
    /// True when the real path of the socket of the engine is a Unix socket.
    pub socket_is_a_socket: bool,
    /// The owner of the real socket, after each link is resolved.
    pub socket_owner: u32,
    /// True when the accepted user of the daemon can connect to the socket of the engine.
    pub accepted_user_can_connect: bool,
    /// Each link and each folder above the socket; not the socket itself, whose mode is the
    /// right to connect.
    pub above_the_socket: Vec<Entry>,
    /// Each link, the real file and each folder above it, of the `docker` program and of the
    /// Compose program.
    pub programs: Vec<Entry>,
    /// The configuration folder of the program for Docker.
    pub configuration: Entry,
    /// The names of the members of `config.json` in that folder; `None` when it has no such
    /// file.
    pub configuration_members: Option<Vec<String>>,
    /// The registries that `auths` of `config.json` names.
    pub logins: Vec<String>,
    pub engine: Version,
    pub compose: Version,
}

/// Why the program refuses to start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StartRefusal {
    #[error("\"Manage applications\" is off")]
    Off,
    #[error("the socket of the engine is not a Unix socket of the machine")]
    NotASocket,
    #[error("the owner of the real socket is not the user that the program runs as")]
    SocketOwner,
    #[error("the kind of the engine is different from the recorded one")]
    KindDifferent,
    #[error("the accepted user is not the owner of the engine of a user")]
    AcceptedUserDifferent,
    #[error("the accepted user of the daemon can connect to the socket of a system engine")]
    AcceptedUserConnects,
    #[error("a user who is not permitted can write a link or a folder above the socket")]
    SocketPathWritable,
    #[error("a user who is not permitted can write a program of Docker or a folder above it")]
    ProgramWritable,
    #[error("a user other than the user of the program can write the configuration folder")]
    ConfigurationWritable,
    #[error("config.json of the configuration folder has a member other than auths")]
    ConfigurationMembers,
    #[error("config.json has a login of a registry that the settings do not name")]
    LoginNotNamed,
    #[error("the engine is older than the lowest version")]
    EngineTooOld,
    #[error("the Compose program is older than the lowest version")]
    ComposeTooOld,
}

/// True when a user who is not in `permitted` can write `entry`: by the owner, by the mode
/// bits or by the access list. The owner counts with each mode, because the owner can change
/// the mode. A group write bit counts unless each member of the group is permitted.
fn can_be_written(entry: &Entry, permitted: &[u32]) -> bool {
    let all_permitted = |users: &[u32]| users.iter().all(|user| permitted.contains(user));
    !permitted.contains(&entry.owner)
        || entry.mode & 0o002 != 0
        || (entry.mode & 0o020 != 0 && !all_permitted(&entry.group_members))
        || entry.listed_for_all
        || !all_permitted(&entry.listed_writers)
}

fn configuration(settings: &Settings, facts: &StartFacts) -> Result<(), StartRefusal> {
    // Only the user that the program runs as can write the configuration folder.
    if can_be_written(&facts.configuration, &[facts.running_as]) {
        return Err(StartRefusal::ConfigurationWritable);
    }
    let members = facts.configuration_members.as_deref().unwrap_or_default();
    if !members.iter().all(|member| member == "auths") {
        return Err(StartRefusal::ConfigurationMembers);
    }
    // A pull uses no login, unless the owner named the registry at the machine.
    if !facts
        .logins
        .iter()
        .all(|login| settings.registries.contains(login))
    {
        return Err(StartRefusal::LoginNotNamed);
    }
    Ok(())
}

/// Decides if the program starts, and with which kind of engine.
///
/// # Errors
/// The first reason of Section 6.4.1 for which the program refuses to start.
pub fn decided(settings: &Settings, facts: &StartFacts) -> Result<EngineKind, StartRefusal> {
    let managed = settings.managed.as_ref().ok_or(StartRefusal::Off)?;
    if !facts.socket_is_a_socket {
        return Err(StartRefusal::NotASocket);
    }
    if facts.socket_owner != facts.running_as {
        return Err(StartRefusal::SocketOwner);
    }
    let kind = if facts.socket_owner == 0 {
        EngineKind::System
    } else {
        EngineKind::User
    };
    if kind != managed.engine {
        return Err(StartRefusal::KindDifferent);
    }
    match kind {
        // On an engine of a user the accepted user is the owner of the socket of the engine.
        EngineKind::User if managed.accepted_user != facts.socket_owner => {
            return Err(StartRefusal::AcceptedUserDifferent);
        }
        // On a system engine the daemon has no right to use Docker (R-226).
        EngineKind::System if facts.accepted_user_can_connect || managed.accepted_user == 0 => {
            return Err(StartRefusal::AcceptedUserConnects);
        }
        _ => {}
    }
    let mut permitted = vec![0, facts.running_as];
    permitted.extend(&facts.administrators);
    if facts
        .above_the_socket
        .iter()
        .any(|entry| can_be_written(entry, &permitted))
    {
        return Err(StartRefusal::SocketPathWritable);
    }
    if facts
        .programs
        .iter()
        .any(|entry| can_be_written(entry, &permitted))
    {
        return Err(StartRefusal::ProgramWritable);
    }
    configuration(settings, facts)?;
    if facts.engine < LOWEST_ENGINE {
        return Err(StartRefusal::EngineTooOld);
    }
    if facts.compose < LOWEST_COMPOSE {
        return Err(StartRefusal::ComposeTooOld);
    }
    Ok(kind)
}

/// True when the owner of the real socket is the one of the start. The program reads the
/// owner again before each item; a different owner ends the program.
#[must_use]
pub const fn socket_owner_is_the_same(at_the_start: u32, now: u32) -> bool {
    at_the_start == now
}
