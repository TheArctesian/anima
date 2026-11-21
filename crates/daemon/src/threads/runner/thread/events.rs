//! What the program of a thread says: each line of its standard output read as events, and
//! what each event changes in the thread. Pure.

use agent_threads::control::Change;
use agent_threads::drive::{DriveEvent, ProgramRequestId, ProgramState};
use uuid::Uuid;

use super::{Driver, Open, Step, Thread, closed, said};

impl Thread {
    /// One line of standard output of the program.
    pub fn line(&mut self, text: &str, new_id: &mut impl FnMut() -> Uuid) -> Vec<Step> {
        let events = self.decode(text.as_bytes());
        let mut steps = Vec::new();
        for event in events {
            steps.extend(self.event(event, text, new_id));
        }
        steps.extend(self.state_change());
        steps
    }

    fn event(
        &mut self,
        event: DriveEvent,
        line: &str,
        new_id: &mut impl FnMut() -> Uuid,
    ) -> Vec<Step> {
        match event {
            DriveEvent::Request(request) => {
                let id = new_id();
                self.open.insert(
                    id,
                    Open {
                        request: request.clone(),
                        line: line.to_owned(),
                    },
                );
                vec![Step::Change(Change::Asked {
                    request: id,
                    detail: request,
                })]
            }
            DriveEvent::RequestClosed { id } => self.program_closed(&id),
            DriveEvent::Reply { id, ok, result } => self.reply(&id, ok, &result),
            DriveEvent::Unknown {
                stops_turn: true, ..
            } => {
                let mut steps = vec![said(event)];
                steps.extend(self.interrupt());
                steps
            }
            other => {
                self.track(&other);
                vec![said(other)]
            }
        }
    }

    /// What an event says of the session, the turn and the state of the program.
    fn track(&mut self, event: &DriveEvent) {
        match event {
            DriveEvent::Session { id, model, .. } => {
                self.row.session = Some(id.clone());
                if model.is_some() {
                    self.row.model.clone_from(model);
                }
            }
            DriveEvent::TurnStarted { turn } => {
                self.program = Some(ProgramState::Working);
                if let Driver::Codex(codex) = &mut self.driver {
                    codex.turn_started(turn.clone());
                }
            }
            DriveEvent::State { state } => self.program = Some(*state),
            DriveEvent::TurnEnded { .. } => {
                self.program = Some(ProgramState::Idle);
                if let Driver::Codex(codex) = &mut self.driver {
                    codex.turn_ended();
                }
            }
            _ => {}
        }
    }

    fn reply(&mut self, id: &ProgramRequestId, ok: bool, result: &serde_json::Value) -> Vec<Step> {
        let (Driver::Codex(codex), ProgramRequestId::Number(number)) = (&mut self.driver, id)
        else {
            return Vec::new();
        };
        match codex.reply(*number, ok, result) {
            Ok(lines) => lines.into_iter().map(Step::Write).collect(),
            Err(fault) => {
                self.last_error = Some(fault.clone());
                vec![said(DriveEvent::Notice { text: fault })]
            }
        }
    }

    fn program_closed(&mut self, id: &ProgramRequestId) -> Vec<Step> {
        let found: Vec<Uuid> = self
            .open
            .iter()
            .filter(|(_, open)| &open.request.id == id)
            .map(|(key, _)| *key)
            .collect();
        found
            .into_iter()
            .map(|key| {
                self.open.remove(&key);
                closed(key, "closed by the program")
            })
            .collect()
    }
}
