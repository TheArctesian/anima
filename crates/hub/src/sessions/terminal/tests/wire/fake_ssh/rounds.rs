//! The fake server's keyboard-interactive rounds: what it asks and when it accepts.

use russh::server::{Auth, Response};

use super::{
    Prompts, USER,
    device::{Device, verdict},
};

impl Device {
    /// The script's prompts in one round.
    fn prompt_count(&self) -> usize {
        match self.script.prompts {
            Prompts::Round(count) => count,
            Prompts::Endless | Prompts::Rounds(_) => 1,
        }
    }

    /// A round of prompts.
    pub(super) fn prompts(&self) -> Auth {
        let prompt = ("Password: ".into(), false);
        Auth::Partial {
            name: "".into(),
            instructions: "".into(),
            prompts: vec![prompt; self.prompt_count()].into(),
        }
    }

    /// The verdict on one round's answers: another round while the script has more.
    pub(super) fn answered_round(&mut self, user: &str, response: Response<'_>) -> Auth {
        let answers: Vec<_> = response.collect();
        let password = self.script.password.as_bytes();
        let right = user == USER
            && answers.len() == self.prompt_count()
            && answers.iter().all(|answer| answer.as_ref() == password);
        self.answered += 1;
        let more = match self.script.prompts {
            Prompts::Rounds(rounds) => self.answered < rounds,
            prompts => prompts == Prompts::Endless,
        };
        if right && more {
            self.prompts()
        } else {
            verdict(right)
        }
    }
}
