//! The requests of a thread that wait for the person (THR-030): one answer for each, bound to
//! the ID of the runner, and closed only by its answer, a stop by the person, the program, or
//! the end of the process. Pure.

use agent_threads::control::{Change, Refusal, RunState};
use agent_threads::drive::{Answer, ClaudeInput, CodexInput, Request};
use uuid::Uuid;

use super::{Driver, Open, Step, Thread, closed};

impl Thread {
    /// A request of the program. A request whose program ID is already open (a line that the
    /// holder gave again, or a program that asks again) keeps the ID of the runner that it has,
    /// so that the program never gets two answers to one request.
    pub(super) fn asked(&mut self, request: Request, line: &str, new_id: Uuid) -> Vec<Step> {
        if let Some(open) = self
            .open
            .values_mut()
            .find(|open| open.request.id == request.id)
        {
            line.clone_into(&mut open.line);
            return Vec::new();
        }
        self.open.insert(
            new_id,
            Open {
                request: request.clone(),
                line: line.to_owned(),
                answering: false,
            },
        );
        vec![Step::Change(Change::Asked {
            request: new_id,
            detail: request,
        })]
    }

    /// The line of the answer of the person to the request `id` of the runner. The request
    /// stays open until the holder took the line ([`Self::answered`]); a second answer before
    /// that is refused.
    ///
    /// # Errors
    /// [`Refusal::RequestGone`] for a request that is answered, closed, being answered or of an
    /// ended process; [`Refusal::AnswerDoesNotFit`] for an answer of the wrong kind.
    pub fn answer(&mut self, id: Uuid, answer: &Answer) -> Result<String, Refusal> {
        if self.row.state == RunState::Stopped || !self.held {
            return Err(Refusal::RequestGone);
        }
        let claude = matches!(self.driver, Driver::Claude(_));
        let open = self
            .open
            .get_mut(&id)
            .filter(|open| !open.answering)
            .ok_or(Refusal::RequestGone)?;
        let line = if claude {
            ClaudeInput::answer(&open.request, answer)
        } else {
            CodexInput::answer(&open.request, answer)
        }
        .map_err(|_| Refusal::AnswerDoesNotFit)?;
        open.answering = true;
        Ok(line)
    }

    /// The holder took the answer to `id`: the request closes.
    pub fn answered(&mut self, id: Uuid) -> Vec<Step> {
        if self.open.remove(&id).is_none() {
            return Vec::new();
        }
        let mut steps = vec![closed(id, "answered by the person")];
        steps.extend(self.state_change());
        steps
    }

    /// The holder did not take the answer to `id`: the request waits for an answer again.
    pub fn answer_failed(&mut self, id: Uuid) {
        if let Some(open) = self.open.get_mut(&id) {
            open.answering = false;
        }
    }

    /// After a restart of the runner: each request whose answer went to the holder closes, as
    /// the answer may have reached the program (THR-030).
    pub fn settle_answering(&mut self) -> Vec<Step> {
        let given: Vec<Uuid> = self
            .open
            .iter()
            .filter(|(_, open)| open.answering)
            .map(|(id, _)| *id)
            .collect();
        if given.is_empty() {
            return Vec::new();
        }
        let mut steps: Vec<Step> = given
            .into_iter()
            .filter_map(|id| {
                self.open
                    .remove(&id)
                    .map(|_| closed(id, "answered before the runner started again"))
            })
            .collect();
        steps.extend(self.state_change());
        steps
    }

    pub(super) fn close_all(&mut self, why: &str) -> Vec<Step> {
        std::mem::take(&mut self.open)
            .into_keys()
            .map(|id| closed(id, why))
            .collect()
    }
}
