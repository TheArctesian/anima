//! Whose unit, and for which hub: the request of `install`, `update`, `remove`, `start` and
//! `stop`, and the unit tool steps that each platform needs.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anima_database::Platform;

use super::steps::Step;
use super::unit_file::{self, LABEL, Launch, SYSTEMD_UNIT};
use crate::Folders;
use crate::members::{Carried, LOCAL_HUB};

/// `launchctl bootstrap` right after `bootout` can fail once.
const BOOTSTRAP_RETRY: Duration = Duration::from_secs(1);

/// Whose unit, and for which hub.
#[derive(Debug, Clone)]
pub struct Request<'a> {
    pub platform: Platform,
    pub uid: u32,
    pub home: &'a Path,
    pub folders: &'a Folders,
    /// Where the hub listens when it is not the default. S5 owns this choice.
    pub listen: Option<SocketAddr>,
    /// How long the running hub may take to stop after the unit was told to stop it.
    pub stop_limit: Duration,
    /// The version of the programs that `source` holds; it is written beside the copy.
    pub version: &'a str,
    /// `install` may put programs of a lower version over those that are installed.
    pub step_back: bool,
}

impl Request<'_> {
    pub(super) fn unit_file(&self) -> PathBuf {
        #[cfg(windows)]
        if self.platform == Platform::Windows {
            return self.folders.root().join("task.xml");
        }
        unit_file::file(self.platform, self.home)
    }

    pub(super) fn launch(&self) -> Launch {
        let database = self.folders.database();
        let mut args = vec!["run".to_owned(), "--root".to_owned()];
        args.push(self.folders.root().to_string_lossy().into_owned());
        args.push("--socket-folder".to_owned());
        args.push(database.socket_folder().to_string_lossy().into_owned());
        if let Some(listen) = self.listen {
            args.extend(["--listen".to_owned(), listen.to_string()]);
        }
        Launch {
            program: Carried::installed_in(&self.folders.programs()).program(LOCAL_HUB),
            args,
        }
    }

    pub(super) fn stop(&self) -> Step {
        match self.platform {
            Platform::MacOs => Step::launchctl(&["bootout", &format!("gui/{}/{LABEL}", self.uid)]),
            Platform::Linux => Step::systemctl(&["stop", SYSTEMD_UNIT]),
            #[cfg(windows)]
            Platform::Windows => self.task("/End").stopping(self.folders.clone()),
        }
        .may_fail()
    }

    pub(super) fn start(&self, file: &Path) -> Vec<Step> {
        match self.platform {
            Platform::MacOs => vec![
                Step::launchctl(&[
                    "bootstrap",
                    &format!("gui/{}", self.uid),
                    &file.to_string_lossy(),
                ])
                .retried_after(BOOTSTRAP_RETRY),
            ],
            #[cfg(windows)]
            Platform::Windows => vec![
                Step::task(&[
                    "/Create",
                    "/TN",
                    &self.task_name(),
                    "/XML",
                    &file.to_string_lossy(),
                    "/F",
                ]),
                self.task("/Run"),
            ],
            Platform::Linux => vec![
                Step::systemctl(&["daemon-reload"]),
                Step::systemctl(&["enable", SYSTEMD_UNIT]),
                Step::systemctl(&["restart", SYSTEMD_UNIT]),
            ],
        }
    }

    /// Succeeds when launchd has the unit loaded.
    pub(super) fn loaded(&self) -> Step {
        Step::launchctl(&["print", &format!("gui/{}/{LABEL}", self.uid)])
    }

    /// Starts the unit that `stop` stopped; it stays enabled for the next login.
    pub(super) fn start_again(&self, file: &Path) -> Vec<Step> {
        match self.platform {
            Platform::MacOs => self.start(file),
            Platform::Linux => vec![Step::systemctl(&["start", SYSTEMD_UNIT])],
            #[cfg(windows)]
            Platform::Windows => vec![self.task("/Run")],
        }
    }

    /// The unit does not start at the next login, and is stopped now.
    pub(super) fn retire(&self) -> Step {
        match self.platform {
            Platform::MacOs => self.stop(),
            Platform::Linux => Step::systemctl(&["disable", "--now", SYSTEMD_UNIT]).may_fail(),
            #[cfg(windows)]
            Platform::Windows => Step::task(&["/Delete", "/TN", &self.task_name(), "/F"])
                .stopping(self.folders.clone())
                .may_fail(),
        }
    }
}

#[cfg(windows)]
impl Request<'_> {
    fn task_name(&self) -> String {
        super::windows::task_name(self.folders.root())
    }
    fn task(&self, action: &str) -> Step {
        Step::task(&[action, "/TN", &self.task_name()])
    }
}
