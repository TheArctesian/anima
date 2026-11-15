//! The stay-awake control of this computer: the operator's choice, the holds that are taken
//! for it and the state the client and the tray show (AWK-001 to AWK-008).

use std::path::PathBuf;

use serde::Serialize;

use super::{
    hold::{self, Hold, LidHold},
    level::{Held, Level, Pause, Power, Wanted, held},
    lid_set_up::{self, Lid, SetUpError},
    platform::Platform,
    power, saved,
};

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
        let mut target = held(self.wanted, power, lid == Lid::Ready, self.was_low);
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

    /// The set-up step of the lid-closed level (macOS).
    ///
    /// # Errors
    /// `SetUp` when this system has no such step, or the administrator step did not finish.
    pub fn set_up_lid(&mut self) -> Result<Status, ControlError> {
        lid_set_up::set_up(&self.platform)?;
        Ok(self.refresh())
    }

    /// Removes the set-up step. The sleep setting is set back first, while the rule still lets it.
    ///
    /// # Errors
    /// As `set_up_lid`.
    pub fn remove_lid_set_up(&mut self) -> Result<Status, ControlError> {
        self.drop_lid();
        let removed = lid_set_up::remove(&self.platform);
        let status = self.refresh();
        removed.map(|()| status).map_err(ControlError::from)
    }

    /// Ends every hold: the computer sleeps as before (AWK-007). The choice stays saved.
    pub fn release(&mut self) {
        self.idle = None;
        self.drop_lid();
    }
}
