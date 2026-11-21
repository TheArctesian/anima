//! The batteries a batch announces: the whole list after start and each time a battery comes,
//! goes or changes its state, not when only its charge or its time left moves (MET-032, K8
//! Section 4). The charge travels as the series `battery_percent`.

use daemon_core::readings::{Battery, BatteryState};
use daemon_core::wire::BatchBattery;

/// The names and states of the battery list last handed to the pusher.
#[derive(Debug, Default)]
pub struct BatteryWatch {
    announced: Option<Vec<(String, BatteryState)>>,
}

impl BatteryWatch {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The `batteries` member to send when the names or states of `read` differ from the list
    /// announced last; `None` when they are the same or nothing could be read (the hub keeps
    /// what it has). An empty list is sent only after a list with a battery: a device with no
    /// battery sends none (K8 Section 4).
    pub fn changed(&mut self, read: Option<&[Battery]>) -> Option<Vec<BatchBattery>> {
        let read = read?;
        let marks: Vec<(String, BatteryState)> = read
            .iter()
            .map(|battery| (battery.name.clone(), battery.state))
            .collect();
        let unchanged = match &self.announced {
            Some(announced) => *announced == marks,
            None => marks.is_empty(),
        };
        if unchanged {
            return None;
        }
        self.announced = Some(marks);
        Some(read.iter().map(BatchBattery::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn battery(name: &str, percent: f64, state: BatteryState) -> Battery {
        Battery {
            name: name.to_owned(),
            percent: Some(percent),
            state,
            seconds_left: Some(9000),
            health_percent: Some(91),
        }
    }

    #[test]
    fn met_032_batteries_are_announced_first_then_only_when_a_name_or_state_changes() {
        let mut watch = BatteryWatch::new();
        let first = [battery("BAT0", 85.0, BatteryState::Discharging)];
        let sent = watch.changed(Some(&first)).expect("announced at start");
        assert_eq!(sent, [BatchBattery::from(&first[0])]);

        let lower = [Battery {
            seconds_left: Some(8000),
            ..battery("BAT0", 84.0, BatteryState::Discharging)
        }];
        assert_eq!(watch.changed(Some(&lower)), None, "charge and time only");
        assert_eq!(watch.changed(None), None, "unread is not empty");

        let plugged = [battery("BAT0", 84.0, BatteryState::Charging)];
        let again = watch.changed(Some(&plugged)).expect("the state changed");
        assert_eq!(again[0].state, daemon_core::wire::BatteryState::Charging);

        let two = [
            battery("BAT0", 84.0, BatteryState::Charging),
            battery("BAT1", 50.0, BatteryState::Charging),
        ];
        assert_eq!(watch.changed(Some(&two)).expect("one came").len(), 2);
        assert_eq!(watch.changed(Some(&[])), Some(Vec::new()), "both gone");
        assert_eq!(watch.changed(Some(&[])), None);
    }

    #[test]
    fn met_032_a_device_with_no_battery_announces_none() {
        let mut watch = BatteryWatch::new();
        assert_eq!(watch.changed(Some(&[])), None);
        assert_eq!(watch.changed(None), None);
    }
}
