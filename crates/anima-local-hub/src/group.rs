//! The hub's group: its members started in order, kept, and stopped in the reverse order
//! (DBL-012). The link, the last member, is kept apart: its end does not end the group (F-6).

use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anima_database::startup::RECOVERED_NOTICE;
use tokio::sync::mpsc;

use crate::child::{Child, ChildError, Spec, Stop, Stopped};
use crate::link_member::{Backoff, LinkMember};
use crate::log_file::{LOG_LIMIT, LogFile};
use crate::members::{self, Carried};
use crate::state::{Blocked, LinkState};
use crate::{Folders, KeyError, vault_key};

/// `anima-database`: another copy runs this database.
const DATABASE_IN_USE: i32 = 10;
/// `anima-database`: the cluster is of another PostgreSQL major version.
const DATABASE_VERSION: i32 = 11;

/// How long each step may take.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// A start after a kill replays the write-ahead log; the database's own limit is 300 s.
    pub database_ready: Duration,
    pub migration: Duration,
    pub gateway_ready: Duration,
    pub hub_ready: Duration,
    /// The hub drains open requests for 10 s.
    pub hub_stop: Duration,
    pub gateway_stop: Duration,
    /// The database's own limit to stop PostgreSQL is 30 s.
    pub database_stop: Duration,
    /// The link's own limit to reach its relay is 30 s.
    pub link_ready: Duration,
    pub link_stop: Duration,
    pub link_backoff: Backoff,
}

/// The limits of the program. Their stop limits sum to less than the 90 s of the unit.
pub const LIMITS: Limits = Limits {
    database_ready: Duration::from_secs(330),
    migration: Duration::from_secs(300),
    gateway_ready: Duration::from_secs(20),
    hub_ready: Duration::from_secs(60),
    hub_stop: Duration::from_secs(20),
    gateway_stop: Duration::from_secs(5),
    database_stop: Duration::from_secs(45),
    link_ready: Duration::from_secs(40),
    link_stop: Duration::from_secs(5),
    link_backoff: Backoff {
        first: Duration::from_secs(10),
        longest: Duration::from_secs(60),
        steady: Duration::from_secs(300),
    },
};

/// What one run of the group works with.
#[derive(Debug, Clone)]
pub struct Plan {
    pub carried: Carried,
    pub folders: Folders,
    /// Where the hub listens. S5 owns this choice; see `members::hub_environment`.
    pub listen: SocketAddr,
    pub limits: Limits,
    /// The time of the link's state changes.
    pub clock: fn() -> SystemTime,
    /// What only a hub of the pool is told, by options of `run` (R-129, R-239). Empty on the
    /// desktop.
    pub pool: members::Pool,
}

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    /// The operator must act; a new start cannot help.
    #[error("the hub cannot start until the operator acts ({0:?})")]
    Blocked(Blocked),
    #[error(transparent)]
    Member(#[from] ChildError),
    #[error(transparent)]
    Key(KeyError),
    #[error("the log of {name} cannot be opened ({kind})")]
    Log {
        name: &'static str,
        kind: io::ErrorKind,
    },
    /// A stop was asked for while the group started.
    #[error("stopped while starting")]
    StopAsked,
}

/// What a started group reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ready {
    pub hub: SocketAddr,
    pub recovered: bool,
    pub link: LinkState,
}

/// Why a ready group is no longer kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    Asked,
    MemberEnded(&'static str),
    /// Not an end: the link changed its state, and the group is kept.
    LinkChanged(LinkState),
}

/// The running members in start order, each with its time to stop.
#[derive(Debug)]
pub struct Group {
    members: Vec<(Child, Duration)>,
    /// Started after the hub; stopped first.
    link: Option<LinkMember>,
    ended: mpsc::UnboundedReceiver<&'static str>,
    report_end: mpsc::UnboundedSender<&'static str>,
}

impl Default for Group {
    fn default() -> Self {
        let (report_end, ended) = mpsc::unbounded_channel();
        Self {
            members: Vec::new(),
            link: None,
            ended,
            report_end,
        }
    }
}

impl Group {
    /// Starts each member when the one before it is ready. The members that did start are
    /// left running when this fails: call [`Group::stop`] in every case.
    ///
    /// # Errors
    /// [`StartError::Blocked`] when a new start cannot help, [`StartError::StopAsked`] as soon
    /// as `stop` is asked, also in the middle of a wait, and the member's error otherwise.
    pub async fn start(&mut self, plan: &Plan, stop: &Stop) -> Result<Ready, StartError> {
        let Plan {
            carried,
            folders,
            limits,
            ..
        } = plan;
        // Before the database: a crash after this leaves a key and no database, never a
        // database that no key opens.
        let database_is_new = match folders.database().cluster().symlink_metadata() {
            Ok(_) => false,
            Err(error) if error.kind() == io::ErrorKind::NotFound => true,
            // A cluster that cannot be looked at is not a new one: no key is made for it.
            Err(error) => return Err(StartError::Key(KeyError::Write(error.kind()))),
        };
        vault_key::prepare(&folders.vault_key(), database_is_new).map_err(|error| match error {
            KeyError::Missing(_) => StartError::Blocked(Blocked::KeyMissing),
            other => StartError::Key(other),
        })?;

        still_wanted(stop)?;
        let database = self.launch(
            &members::database(carried, folders),
            plan,
            limits.database_stop,
        )?;
        let mut recovered = false;
        let ready = database.line_where(limits.database_ready, |line| {
            recovered |= line == RECOVERED_NOTICE;
            (line == "ready").then_some(())
        });
        unless_stopped(stop, ready)
            .await
            .map_err(|error| match error {
                StartError::Member(ended) => database_refusal(ended),
                other => other,
            })?;
        database.stop_reading();

        migrate(plan, stop).await?;

        let gateway = self.launch(&members::gateway(carried), plan, limits.gateway_stop)?;
        let ready = gateway.line_where(limits.gateway_ready, members::gateway_address);
        let gateway_address = unless_stopped(stop, ready).await?;
        gateway.stop_reading();

        let hub = members::hosted(
            members::hub(carried, folders, plan.listen, gateway_address),
            &plan.pool,
        );
        let hub = self.launch(&hub, plan, limits.hub_stop)?;
        let ready = hub.line_where(limits.hub_ready, members::hub_address);
        let hub_address = unless_stopped(stop, ready).await?;
        hub.stop_reading();

        // The group is ready without it: the hub serves this computer while the link waits.
        let link = members::link(carried, folders);
        let link = LinkMember::start(link, log_path(plan, members::LINK), *limits, plan.clock);
        let link = self.link.insert(link).state();
        Ok(Ready {
            hub: hub_address,
            recovered,
            link,
        })
    }

    fn launch(
        &mut self,
        spec: &Spec,
        plan: &Plan,
        stop_limit: Duration,
    ) -> Result<&mut Child, StartError> {
        let child = Child::start(spec, log(plan, spec.name)?, self.report_end.clone())?;
        self.members.push((child, stop_limit));
        match self.members.last_mut() {
            Some((child, _)) => Ok(child),
            None => unreachable!("a member was pushed"),
        }
    }

    /// Returns when `stop` is asked for, a member other than the link ends by itself, or the
    /// link changes its state. Cancel-safe.
    pub async fn until_ended(&mut self, stop: &mut Stop) -> Ending {
        let link = self.link.as_mut();
        let link_changed = async move {
            match link {
                Some(link) => link.changed().await,
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            () = stop.asked() => Ending::Asked,
            name = self.ended.recv() => Ending::MemberEnded(name.unwrap_or("a member")),
            link = link_changed => Ending::LinkChanged(link),
        }
    }

    /// Stops the members, the last one started first. A member that does not end in its
    /// time is killed. Returns one line for each member that did not end by itself.
    pub async fn stop(mut self) -> Vec<String> {
        let mut notes = match self.link.take() {
            Some(link) => link.stop().await,
            None => Vec::new(),
        };
        for (child, limit) in self.members.into_iter().rev() {
            let name = child.name();
            match child.stop(limit).await {
                Ok(Stopped::Ended(_)) => {}
                Ok(Stopped::Killed) => notes.push(format!(
                    "{name} did not stop in {} s and was killed",
                    limit.as_secs()
                )),
                // It is left to the unit's own kill.
                Err(error) => notes.push(error.to_string()),
            }
        }
        notes
    }
}

/// `wait`, unless a stop is asked for first. The member that `wait` is for keeps running;
/// [`Group::stop`] ends it.
async fn unless_stopped<T>(
    stop: &Stop,
    wait: impl Future<Output = Result<T, ChildError>>,
) -> Result<T, StartError> {
    let mut stop = stop.clone();
    tokio::select! {
        biased;
        done = wait => Ok(done?),
        () = stop.asked() => Err(StartError::StopAsked),
    }
}

fn still_wanted(stop: &Stop) -> Result<(), StartError> {
    if stop.is_asked() {
        return Err(StartError::StopAsked);
    }
    Ok(())
}

fn database_refusal(error: ChildError) -> StartError {
    match error.exit_code() {
        Some(DATABASE_IN_USE) => StartError::Blocked(Blocked::DatabaseInUse),
        Some(DATABASE_VERSION) => StartError::Blocked(Blocked::DatabaseVersion),
        _ => StartError::Member(error),
    }
}

fn log_path(plan: &Plan, name: &str) -> PathBuf {
    // "anima-hub migrate" and "anima-hub" share the hub's log.
    let file = name.split(' ').next().unwrap_or(name);
    plan.folders.logs().join(format!("{file}.log"))
}

fn log(plan: &Plan, name: &'static str) -> Result<LogFile, StartError> {
    LogFile::open(&log_path(plan, name), LOG_LIMIT).map_err(|error| StartError::Log {
        name,
        kind: error.kind(),
    })
}

/// What a migration that must be given up gets between SIGTERM and SIGKILL. A migration is
/// one transaction, so the database loses nothing.
const MIGRATION_STOP: Duration = Duration::from_secs(5);

/// Runs `anima-hub migrate` to its end. One that runs past its limit, or that a stop request
/// finds running, is stopped.
async fn migrate(plan: &Plan, stop: &Stop) -> Result<(), StartError> {
    let spec = members::migration(&plan.carried, &plan.folders);
    // Its end is awaited here, so nobody else is told of it.
    let (report_end, _ended) = mpsc::unbounded_channel();
    let mut migration = Child::start(&spec, log(plan, spec.name)?, report_end)?;
    let ended = unless_stopped(stop, migration.end_within(plan.limits.migration)).await;
    match ended {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(ChildError::Ended {
            name: spec.name,
            status,
        }
        .into()),
        Err(given_up) => {
            let _ = migration.stop(MIGRATION_STOP).await;
            Err(given_up)
        }
    }
}
