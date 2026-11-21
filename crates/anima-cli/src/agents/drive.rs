//! `anima agents start`, `send`, `approve`, `interrupt`, `stop`, `running`, `watch` and
//! `raise-limit`: the person drives the agent threads of this computer through the runner of
//! the daemon (TH13, THR-030, THR-049, THR-114). No MCP tool starts a thread or answers a
//! request: an answer is a trust decision, and a start waits for the count of THR-114
//! (Section 7 of cli-mcp-plan.md).

use std::path::PathBuf;

use agent_threads::control::{Action, OVER_LIMIT, Outcome, Refusal, Reply, ThreadRow};
use agent_threads::drive::{Answer, Effort, Mode, Program};
use uuid::Uuid;

use super::runner::{RunnerLink, refusal};
use crate::failure::Failure;
use crate::invocation::Invocation;
use crate::output::stream::{self, Reader};
use crate::output::{Cell, Table, Tone};

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum ProgramName {
    ClaudeCode,
    Codex,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum ModeName {
    /// The program asks nothing (the default, R-308).
    FullAccess,
    /// The program changes files in the folder without asking and asks for the rest.
    AcceptEdits,
    /// The program asks before each change.
    Ask,
}

#[derive(Debug, clap::Args)]
pub struct StartArgs {
    #[arg(long, value_enum)]
    program: ProgramName,
    /// The folder the agent works in: an absolute folder of this computer.
    #[arg(long)]
    folder: PathBuf,
    #[arg(long, value_enum, default_value = "full-access")]
    mode: ModeName,
    #[arg(long)]
    model: Option<String>,
    #[arg(long, value_parser = parse_effort)]
    effort: Option<Effort>,
    /// The first prompt.
    prompt: String,
}

#[derive(Debug, clap::Args)]
pub struct SendArgs {
    thread: Uuid,
    /// The prompt: to the running turn, or as a new turn.
    text: String,
}

#[derive(Debug, clap::Args)]
pub struct ApproveArgs {
    thread: Uuid,
    /// The ID of the request, as `anima agents watch` shows it.
    request: Uuid,
    /// Deny, with this message to the agent.
    #[arg(long, conflicts_with_all = ["for_session", "choose"])]
    deny: Option<String>,
    /// Allow, and send back the session rule at this place of the request (0 is the first).
    #[arg(long, conflicts_with = "choose")]
    for_session: Option<usize>,
    /// The answer to a question, as KEY=CHOICE; once for each choice.
    #[arg(long, value_parser = parse_choice)]
    choose: Vec<(String, String)>,
}

#[derive(Debug, clap::Args)]
pub struct ThreadIdArgs {
    thread: Uuid,
}

#[derive(Debug, clap::Args)]
pub struct WatchArgs {
    /// Only the updates of this thread.
    thread: Option<Uuid>,
}

fn parse_effort(text: &str) -> Result<Effort, String> {
    serde_json::from_value(serde_json::Value::from(text))
        .map_err(|_| "low, medium, high, xhigh or max".to_owned())
}

fn parse_choice(text: &str) -> Result<(String, String), String> {
    text.split_once('=')
        .map(|(key, choice)| (key.to_owned(), choice.to_owned()))
        .ok_or_else(|| "KEY=CHOICE".to_owned())
}

/// Gives a result that is not a refusal.
pub(super) fn accepted(outcome: Outcome) -> Result<Outcome, Failure> {
    match outcome {
        Outcome::Refused { why } => Err(refusal(&why)),
        other => Ok(other),
    }
}

/// Said when the project files of the folder could not be checked before a start.
const NOT_CHECKED: &str = "The project files of this folder could not be checked: they can make Claude Code run commands with no question.";

impl StartArgs {
    /// Starts a thread. At the soft limit the person is asked whether to raise it (THR-114).
    ///
    /// # Errors
    /// [`Failure`] for a refusal of the runner, a no at the limit, or a runner that is not there.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let mut link = RunnerLink::connect().await?;
        let folder = std::path::absolute(&self.folder).unwrap_or(self.folder);
        // What the files of the folder make the program do by itself, before the prompt goes
        // (THR-046, OD-THR21: honoured, with a notice). A check that fails is said and does
        // not stop the start.
        let checked = link
            .call(Action::Folder {
                folder: folder.clone(),
            })
            .await;
        match checked.map(accepted) {
            Ok(Ok(Outcome::Folder { notice })) if notice.is_empty() => {}
            Ok(Ok(Outcome::Folder { notice })) => invocation.output.message(&notice.join("\n")),
            _ => invocation.output.message(NOT_CHECKED),
        }
        let action = Action::Start {
            program: match self.program {
                ProgramName::ClaudeCode => Program::ClaudeCode,
                ProgramName::Codex => Program::Codex,
            },
            folder,
            mode: match self.mode {
                ModeName::FullAccess => Mode::FullAccess,
                ModeName::AcceptEdits => Mode::AcceptEdits,
                ModeName::Ask => Mode::Ask,
            },
            model: self.model,
            effort: self.effort,
            prompt: self.prompt,
        };
        loop {
            match link.call(action.clone()).await? {
                Outcome::Started { thread } => {
                    return invocation
                        .output
                        .data(&thread, |thread| one(&thread.to_string()));
                }
                Outcome::Refused {
                    why: Refusal::OverLimit { .. },
                } if invocation.prompter.interactive() => {
                    invocation.prompter.confirm(OVER_LIMIT)?;
                    accepted(link.call(Action::RaiseLimit).await?)?;
                }
                other => {
                    accepted(other)?;
                    return Ok(());
                }
            }
        }
    }
}

fn one(text: &str) -> Table {
    let mut table = Table::new(vec!["THREAD"]);
    table.row(vec![Cell::new(text)]);
    table
}

/// Sends one call whose answer is only that it was done.
async fn simple(invocation: &Invocation, action: Action, done: &str) -> Result<(), Failure> {
    let mut link = RunnerLink::connect().await?;
    if let Outcome::Stopped { all_ended: false } = accepted(link.call(action).await?)? {
        invocation.output.message(
            "The program ended, but a process below it could not be proved to have ended.",
        );
    } else {
        invocation.output.message(done);
    }
    Ok(())
}

impl SendArgs {
    /// # Errors
    /// [`Failure`] for a refusal of the runner.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let action = Action::Send {
            thread: self.thread,
            text: self.text,
        };
        simple(invocation, action, "Sent.").await
    }
}

impl ApproveArgs {
    /// # Errors
    /// [`Failure`] for a refusal of the runner.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let answer = match (self.deny, self.for_session, self.choose.is_empty()) {
            (Some(message), _, _) => Answer::Deny { message },
            (None, Some(rule), _) => Answer::AllowForSession { rule },
            (None, None, false) => Answer::Choose {
                answers: self.choose,
            },
            (None, None, true) => Answer::Allow,
        };
        let action = Action::Answer {
            thread: self.thread,
            request: self.request,
            answer,
        };
        simple(invocation, action, "Answered.").await
    }
}

/// `anima agents interrupt`: the running turn stops; the program stays.
///
/// # Errors
/// [`Failure`] for a refusal of the runner.
pub async fn interrupt(args: ThreadIdArgs, invocation: &Invocation) -> Result<(), Failure> {
    simple(
        invocation,
        Action::Interrupt {
            thread: args.thread,
        },
        "The turn stopped.",
    )
    .await
}

/// `anima agents stop`: the program and each process below it end.
///
/// # Errors
/// [`Failure`] for a refusal of the runner.
pub async fn stop(args: ThreadIdArgs, invocation: &Invocation) -> Result<(), Failure> {
    simple(
        invocation,
        Action::Stop {
            thread: args.thread,
        },
        "The program and its processes ended.",
    )
    .await
}

/// `anima agents raise-limit`: the person raises the soft limit one step (THR-114).
///
/// # Errors
/// [`Failure::Declined`] for a no; a refusal of the runner.
pub async fn raise_limit(invocation: &Invocation) -> Result<(), Failure> {
    invocation.prompter.confirm(OVER_LIMIT)?;
    let mut link = RunnerLink::connect().await?;
    if let Outcome::Limits { limits } = accepted(link.call(Action::RaiseLimit).await?)? {
        invocation.output.message(&format!(
            "The limits are now {} deep, {} children of one thread and {} threads at one time.",
            limits.depth, limits.children, limits.running
        ));
    }
    Ok(())
}

/// `anima agents running`: the threads the runner started, with their state.
///
/// # Errors
/// [`Failure`] when the runner is not there.
pub async fn running(invocation: &Invocation) -> Result<(), Failure> {
    let mut link = RunnerLink::connect().await?;
    let Outcome::Threads { threads } = accepted(link.call(Action::List).await?)? else {
        return Ok(());
    };
    invocation.output.data(&threads, |threads| table(threads))
}

fn table(rows: &[ThreadRow]) -> Table {
    let mut table = Table::new(vec!["STATE", "PROGRAM", "TITLE", "FOLDER", "THREAD"]);
    for row in rows {
        let state = serde_json::to_value(row.state).ok();
        table.row(vec![
            Cell::toned(
                state.as_ref().and_then(|v| v.as_str()).unwrap_or("-"),
                Tone::Muted,
            ),
            Cell::new(super::threads::shown::program_words(program_key(
                row.program,
            ))),
            Cell::new(&row.title),
            Cell::new(&row.folder.display().to_string()),
            Cell::new(&row.id.to_string()),
        ]);
    }
    table
}

pub(super) const fn program_key(program: Program) -> &'static str {
    match program {
        Program::ClaudeCode => "claude_code",
        Program::Codex => "codex",
    }
}

/// `anima agents watch`: the updates of the threads as they happen, one line each.
///
/// # Errors
/// [`Failure`] when the runner is not there or standard output fails.
pub async fn watch(args: WatchArgs, invocation: &Invocation) -> Result<(), Failure> {
    let mut link = RunnerLink::connect().await?;
    link.send(Action::Watch).await?;
    while let Some(reply) = link.next().await? {
        let Reply::Update(update) = reply else {
            continue;
        };
        if args.thread.is_some_and(|thread| thread != update.thread) {
            continue;
        }
        let Some(text) = super::watch_text::text(&update.change) else {
            continue;
        };
        let thread = update.thread.to_string();
        let shown = format!("{} {text}", &thread[..8]);
        if stream::emit(invocation.output.json(), &*update, |_| shown)? == Reader::Gone {
            break;
        }
    }
    Ok(())
}
