//! `anima-database`: wires the command line to the library and sets the exit code. No rules here.
//!
//!   anima-database serve  --tree <folder> [--root <folder> --socket-folder <folder>]
//!   anima-database status --tree <folder> [--root <folder> --socket-folder <folder>]
//!
//! `serve` prints `ready` on standard output when the database accepts connections, and stops
//! the server on SIGTERM or SIGINT. Nothing it prints holds a password (LOG-007).

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anima_database::startup::{self, Running, StartError};
use anima_database::{
    Layout, LayoutError, Limits, Platform, ServerError, Settings, Tree, UserFolders, cluster,
};
use clap::{Args, Parser, Subcommand};
use tokio::signal::unix::{Signal, SignalKind, signal};

/// Another copy runs this database.
const EXIT_ANOTHER_COPY: u8 = 10;
/// The server ended by itself.
const EXIT_SERVER_ENDED: u8 = 3;

const SETTINGS: Settings = Settings {
    max_connections: 30,
    shared_buffers_mb: 32,
    // D-4: a Mac needs it to keep the cluster whole through a power cut.
    flush_through_drive_cache: matches!(Platform::HOST, Platform::MacOs),
};
const LIMITS: Limits = Limits {
    // A start after a kill replays the write-ahead log first.
    ready: Duration::from_secs(300),
    stop: Duration::from_secs(30),
};

#[derive(Parser)]
#[command(
    name = "anima-database",
    about = "The PostgreSQL that the Anima application carries"
)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Bring the database up and keep it up until SIGTERM
    Serve(Place),
    /// Print the versions and whether a copy runs
    Status(Place),
}

#[derive(Args)]
struct Place {
    /// The PostgreSQL tree: a folder with bin/, lib/ and share/
    #[arg(long)]
    tree: PathBuf,
    /// The database root; the default is the application's folder of this user
    #[arg(long, requires = "socket_folder")]
    root: Option<PathBuf>,
    /// The folder for the Unix socket
    #[arg(long, requires = "root")]
    socket_folder: Option<PathBuf>,
}

impl Place {
    fn layout(&self) -> Result<Layout, LayoutError> {
        match (&self.root, &self.socket_folder) {
            (Some(root), Some(socket_folder)) => Layout::new(root.clone(), socket_folder.clone()),
            _ => Layout::for_user(
                Platform::HOST,
                &UserFolders {
                    home: std::env::var_os("HOME")
                        .map(PathBuf::from)
                        .unwrap_or_default(),
                    xdg_data_home: std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
                    xdg_runtime_dir: std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from),
                },
            ),
        }
    }
}

fn main() -> ExitCode {
    let outcome = match Arguments::parse().command {
        Command::Serve(place) => serve(&place),
        Command::Status(place) => status(&place),
    };
    match outcome {
        Ok(code) => code,
        Err(error) => {
            eprintln!("anima-database: {}", with_causes(error.as_ref()));
            let another_copy = matches!(
                error.downcast_ref::<StartError>(),
                Some(StartError::Server(ServerError::AnotherCopy))
            );
            ExitCode::from(if another_copy { EXIT_ANOTHER_COPY } else { 1 })
        }
    }
}

type Failure = Box<dyn std::error::Error>;

fn serve(place: &Place) -> Result<ExitCode, Failure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    // Before the server starts, so a signal during the start still stops it cleanly.
    let (terminate, interrupt) = {
        let _context = runtime.enter();
        (
            signal(SignalKind::terminate())?,
            signal(SignalKind::interrupt())?,
        )
    };
    let tree = Tree::at(&place.tree)?;
    let layout = place.layout()?;
    let mut running = startup::start(&tree, &layout, SETTINGS, LIMITS)?;
    if running.recovered {
        eprintln!("anima-database: recovered after an unclean stop");
    }
    println!("ready");
    std::io::stdout().flush()?;
    let ended = runtime.block_on(asked_to_stop_or_ended(&mut running, terminate, interrupt))?;
    if let Some(status) = ended {
        eprintln!("anima-database: the server ended by itself ({status})");
        return Ok(ExitCode::from(EXIT_SERVER_ENDED));
    }
    running.stop(LIMITS)?;
    Ok(ExitCode::SUCCESS)
}

/// `None` when a signal asks for a stop; the exit status when the server ends by itself.
async fn asked_to_stop_or_ended(
    running: &mut Running,
    mut terminate: Signal,
    mut interrupt: Signal,
) -> Result<Option<std::process::ExitStatus>, ServerError> {
    let mut look = tokio::time::interval(Duration::from_millis(200));
    loop {
        tokio::select! {
            _ = terminate.recv() => return Ok(None),
            _ = interrupt.recv() => return Ok(None),
            _ = look.tick() => {
                if let Some(status) = running.ended()? {
                    return Ok(Some(status));
                }
            }
        }
    }
}

fn status(place: &Place) -> Result<ExitCode, Failure> {
    let tree = Tree::at(&place.tree)?;
    let layout = place.layout()?;
    let of_cluster = cluster::major_version(&layout)?;
    println!("tree: PostgreSQL {}", tree.major_version()?);
    match of_cluster {
        Some(version) => println!("cluster: PostgreSQL {version}"),
        None => println!("cluster: none"),
    }
    let runs = of_cluster.is_some()
        && matches!(anima_database::lock(&layout), Err(ServerError::AnotherCopy));
    println!("running: {}", if runs { "yes" } else { "no" });
    Ok(ExitCode::SUCCESS)
}

/// The error and each cause below it, on one line.
fn with_causes(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(below) = cause {
        text.push_str(": ");
        text.push_str(&below.to_string());
        cause = below.source();
    }
    text
}
