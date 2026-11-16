//! The stay-awake control of this computer: the operator's choice, the holds that are taken
//! for it and the state the client and the tray show (AWK-001 to AWK-008).

use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use serde::Serialize;

use super::{
    hold::{self, Hold, LidHold},
    level::{Held, Level, Pause, Power, Wanted, held},
    lid_set_up::{self, Lid, SetUpError},
    platform::{Os, Platform},
    power, saved,
};

/// After a refused administrator step, the next one is refused with no dialog for this long,
/// so a script in the web view cannot show the operator one password dialog after another.
const REFUSAL_REST: Duration = Duration::from_secs(15);

/// The two administrator steps of the lid-closed level (macOS).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    SetUp,
    Remove,
}

/// One administrator step, ready to run. It runs with no lock on the control: the password
/// dialog can stay open for minutes, and the app must still be able to end its holds.
#[derive(Debug)]
pub struct Ask {
    platform: Platform,
    step: Step,
    /// Anima changed the sleep setting and could not set it back before a `Remove`.
    set_sleep_back: bool,
}

impl Ask {
    /// Shows the dialog and waits for it.
    ///
    /// # Errors
    /// `Refused` when the dialog was cancelled or the script failed.
    pub fn run(&self) -> Result<(), SetUpError> {
        match self.step {
            Step::SetUp => lid_set_up::set_up(&self.platform),
            Step::Remove => lid_set_up::remove(&self.platform, self.set_sleep_back),
        }
    }
}

/// What the client and the tray show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// The operator's choice.
    pub level: Level,
    pub battery_floor: Option<u8>,
    /// The level the computer is held at now.
    pub held: Level,
    /// Why `held` is lower than `level`.
    pub pause: Option<Pause>,
    pub lid: Lid,
}

/// Why a change was not made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ControlError {
    #[error("the choice could not be saved")]
    Unsaved,
    #[error(transparent)]
    SetUp(#[from] SetUpError),
}

#[derive(Debug)]
pub struct Control {
    platform: Platform,
    /// The app's configuration folder: the saved choice and the marker.
    folder: PathBuf,
    wanted: Wanted,
    was_low: bool,
    idle: Option<Hold>,
    lid: Option<LidHold>,
    /// An administrator dialog is open.
    asking: bool,
    refused_at: Option<Instant>,
}

impl Control {
    /// The control with the saved choice and no hold yet. A sleep setting that an earlier run
    /// of the app left changed is set back first (AWK-007).
    #[must_use]
    pub fn open(platform: Platform, folder: PathBuf) -> Self {
        // A failure keeps the marker, so the next start tries again.
        let _ = hold::restore(&platform, &saved::marker(&folder));
        Self {
            wanted: saved::load(&folder),
            platform,
            folder,
            was_low: false,
            idle: None,
            lid: None,
            asking: false,
            refused_at: None,
        }
    }

    /// Whether a level is chosen, so the holds need a look from time to time.
    #[must_use]
    pub fn active(&self) -> bool {
        self.wanted.level != Level::Off
    }

    /// The power source, read only when a floor can change the answer.
    fn power(&self) -> Power {
        if self.active() && self.wanted.battery_floor.is_some() {
            power::read(&self.platform)
        } else {
            Power::Mains
        }
    }

    fn hold_idle(&mut self) -> bool {
        if self.idle.as_mut().is_some_and(Hold::alive) {
            return true;
        }
        self.idle = hold::idle(&self.platform).ok();
        self.idle.is_some()
    }

    fn lid_holds(&mut self) -> bool {
        match &mut self.lid {
            Some(LidHold::Inhibitor(hold) | LidHold::Setting { watcher: hold }) => hold.alive(),
            // The other program can set its setting back at any time: look again.
            Some(LidHold::Borrowed) | None => false,
        }
    }

    fn hold_lid(&mut self) -> bool {
        if self.lid_holds() {
            return true;
        }
        self.lid = hold::lid(&self.platform, &saved::marker(&self.folder)).ok();
        self.lid.is_some()
    }

    fn drop_lid(&mut self) {
        if let Some(lid) = self.lid.take() {
            // A failure keeps the marker; the next start sets the setting back.
            let _ = hold::release_lid(&self.platform, &saved::marker(&self.folder), lid);
        }
    }

    /// Takes and ends holds for the choice on `power`, and gives the state that results.
    pub fn apply(&mut self, power: Power) -> Status {
        let lid = lid_set_up::lid(&self.platform);
        // No lid hold is taken while an administrator step runs: the rule can go at any moment.
        let lid_ready = lid == Lid::Ready && !self.asking;
        let mut target = held(self.wanted, power, lid_ready, self.was_low);
        if target.level == Level::LidClosed && !self.hold_lid() {
            target = held(self.wanted, power, false, self.was_low);
        }
        if target.level != Level::LidClosed {
            self.drop_lid();
        }
        if target.level == Level::Off {
            self.idle = None;
        } else if !self.hold_idle() {
            self.drop_lid();
            target = Held {
                level: Level::Off,
                pause: Some(Pause::Unavailable),
            };
        }
        self.was_low = target.pause == Some(Pause::LowBattery);
        Status {
            level: self.wanted.level,
            battery_floor: self.wanted.battery_floor,
            held: target.level,
            pause: target.pause,
            lid,
        }
    }

    /// `apply` on the power source of this moment.
    pub fn refresh(&mut self) -> Status {
        self.apply(self.power())
    }

    fn change(&mut self, wanted: Wanted) -> Result<Status, ControlError> {
        saved::save(&self.folder, wanted).map_err(|_| ControlError::Unsaved)?;
        self.wanted = wanted;
        // The operator's own change ends a low-battery pause; the charge decides again.
        self.was_low = false;
        Ok(self.refresh())
    }

    /// The operator chose `level`.
    ///
    /// # Errors
    /// `Unsaved` when the choice could not be saved; nothing changes then.
    pub fn choose(&mut self, level: Level) -> Result<Status, ControlError> {
        self.change(Wanted {
            level,
            ..self.wanted
        })
    }

    /// The operator set or removed the battery floor.
    ///
    /// # Errors
    /// As `choose`.
    pub fn set_floor(&mut self, floor: Option<u8>) -> Result<Status, ControlError> {
        self.change(self.wanted.with_floor(floor))
    }

    /// Starts an administrator step. `None`: there is nothing to ask, the computer is already
    /// in the state the step gives. A `Remove` sets the sleep setting back first, while the
    /// rule still lets it.
    ///
    /// # Errors
    /// `SetUp(Unsupported)` off macOS; `SetUp(Refused)` while a dialog is open, and for a short
    /// time after a refused step.
    pub fn begin(&mut self, step: Step, now: Instant) -> Result<Option<Ask>, ControlError> {
        if self.platform.os != Os::Mac {
            return Err(SetUpError::Unsupported.into());
        }
        let resting = self
            .refused_at
            .is_some_and(|at| now.duration_since(at) < REFUSAL_REST);
        if self.asking || resting {
            return Err(SetUpError::Refused.into());
        }
        let set_up = lid_set_up::lid(&self.platform) == Lid::Ready;
        if set_up == (step == Step::SetUp) {
            return Ok(None);
        }
        self.asking = true;
        if step == Step::Remove {
            self.drop_lid();
        }
        Ok(Some(Ask {
            platform: self.platform.clone(),
            step,
            set_sleep_back: saved::marker(&self.folder).exists(),
        }))
    }

    /// Ends the administrator step `ask` with what its dialog answered.
    ///
    /// # Errors
    /// `SetUp(Refused)` when the step did not finish; the state is as before the step.
    pub fn finish(
        &mut self,
        ask: &Ask,
        asked: Result<(), SetUpError>,
        now: Instant,
    ) -> Result<Status, ControlError> {
        self.asking = false;
        self.refused_at = asked.is_err().then_some(now);
        if asked.is_ok() && ask.set_sleep_back {
            // Root set the sleep setting back with the rule's removal.
            let _ = fs::remove_file(saved::marker(&self.folder));
        }
        let status = self.refresh();
        asked.map(|()| status).map_err(ControlError::from)
    }

    /// Ends every hold: the computer sleeps as before (AWK-007). The choice stays saved.
    pub fn release(&mut self) {
        self.idle = None;
        self.drop_lid();
    }
}
