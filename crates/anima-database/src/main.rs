//! `anima-database`: wires the command line to the library and sets the exit code. No rules here.
//!
//!   anima-database serve  --tree <folder> [--root <folder> --socket-folder <folder>]
//!   anima-database status --tree <folder> [--root <folder> --socket-folder <folder>]
//!   anima-database backup  <file> --tree <folder> [...]
//!   anima-database restore <file> --tree <folder> [...]
//!   anima-database verify  --tree <folder> [...]
//!
//! `backup`, `restore` and `verify` need a running `serve`.
//!
//! `serve` prints `ready` on standard output when the database accepts connections, and stops
//! the server on SIGTERM or SIGINT. Nothing it prints holds a password (LOG-007).

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use anima_database::backup::Checked;
use anima_database::startup::{self, BackupExclusion, Running, StartError};
use anima_database::{
    Layout, LayoutError, Limits, Platform, ServerError, Settings, Tree, UserFolders,
};
use clap::{Args, Parser, Subcommand};
mod stopping;

/// Another copy runs this database.
const EXIT_ANOTHER_COPY: u8 = 10;
/// The cluster is of another PostgreSQL major version and was not touched.
const EXIT_OTHER_VERSION: u8 = 11;
/// The server ended by itself.
const EXIT_SERVER_ENDED: u8 = 3;
/// `verify` found damage.
const EXIT_DAMAGED: u8 = 4;

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
    /// Print the versions and whether a server accepts connections
    Status(Place),
    /// Write a backup of the hub's database to a file (mode 0600)
    Backup {
        file: PathBuf,
        #[command(flatten)]
        place: Place,
    },
    /// Restore a backup into the hub's database, which must be empty
    Restore {
        file: PathBuf,
        #[command(flatten)]
        place: Place,
    },
    /// Check every table and index for damage
    Verify(Place),
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
                    home: std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                        .map_or_else(PathBuf::new, PathBuf::from),
                    #[cfg(windows)]
                    local_app_data: std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
                    xdg_data_home: std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
                    xdg_runtime_dir: std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from),
                },
            ),
        }
    }
}

fn main() -> ExitCode {
    // A core file of this program could hold a password; none is written.
    #[cfg(unix)]
    nix::sys::resource::setrlimit(nix::sys::resource::Resource::RLIMIT_CORE, 0, 0).ok();
    let outcome = match Arguments::parse().command {
        Command::Serve(place) => serve(&place),
        Command::Status(place) => status(&place),
        Command::Backup { file, place } => backup(&place, &file),
        Command::Restore { file, place } => restore(&place, &file),
        Command::Verify(place) => verify(&place),
    };
    match outcome {
        Ok(code) => code,
        Err(error) => {
            eprintln!("anima-database: {}", with_causes(error.as_ref()));
            let start = error.downcast_ref::<StartError>();
            ExitCode::from(match start {
                Some(refusal) if refusal.is_another_copy() => EXIT_ANOTHER_COPY,
                Some(refusal) if refusal.is_version_refusal() => EXIT_OTHER_VERSION,
                _ => 1,
            })
        }
    }
}

type Failure = Box<dyn std::error::Error>;

fn serve(place: &Place) -> Result<ExitCode, Failure> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    // Before the server starts: a signal that comes during the start is kept, and stops the
    // server cleanly as soon as the start is done. It does not cut the start short.
    let stop = {
        let _context = runtime.enter();
        stopping::Requested::new()?
    };
    let tree = Tree::at(&place.tree)?;
    let layout = place.layout()?;
    let mut running = startup::start(&tree, &layout, SETTINGS, LIMITS)?;
    if running.recovered {
        eprintln!("{}", startup::RECOVERED_NOTICE);
    }
    if running.backup_exclusion == BackupExclusion::Failed {
        eprintln!("anima-database: the cluster could not be excluded from Time Machine");
    }
    println!("ready");
    std::io::stdout().flush()?;
    let ended = runtime.block_on(asked_to_stop_or_ended(&mut running, stop))?;
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
    mut stop: stopping::Requested,
) -> Result<Option<std::process::ExitStatus>, ServerError> {
    let mut look = tokio::time::interval(Duration::from_millis(200));
    loop {
        tokio::select! {
            () = stop.wait() => return Ok(None),
            _ = look.tick() => {
                if let Some(status) = running.ended()? {
                    return Ok(Some(status));
                }
            }
        }
    }
}

fn status(place: &Place) -> Result<ExitCode, Failure> {
    let status = startup::status(&Tree::at(&place.tree)?, &place.layout()?)?;
    println!("tree: PostgreSQL {}", status.tree);
    match status.cluster {
        Some(version) => println!("cluster: PostgreSQL {version}"),
        None => println!("cluster: none"),
    }
    let server = if status.server_accepts_connections {
        "accepts connections"
    } else {
        "not running"
    };
    println!("server: {server}");
    Ok(ExitCode::SUCCESS)
}

fn backup(place: &Place, file: &Path) -> Result<ExitCode, Failure> {
    anima_database::backup::write(&Tree::at(&place.tree)?, &place.layout()?, file)?;
    Ok(ExitCode::SUCCESS)
}

fn restore(place: &Place, file: &Path) -> Result<ExitCode, Failure> {
    anima_database::backup::restore(&Tree::at(&place.tree)?, &place.layout()?, file)?;
    Ok(ExitCode::SUCCESS)
}

fn verify(place: &Place) -> Result<ExitCode, Failure> {
    match anima_database::backup::check(&Tree::at(&place.tree)?, &place.layout()?)? {
        Checked::Sound => {
            println!("sound");
            Ok(ExitCode::SUCCESS)
        }
        Checked::Damaged(names) => {
            for name in names {
                println!("damaged: {name}");
            }
            Ok(ExitCode::from(EXIT_DAMAGED))
        }
    }
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
