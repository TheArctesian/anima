//! Throwaway git repositories for `anima snapshot`: a working repository, a bare repository on
//! disk as its `origin`, and git with no global or system configuration of this computer.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tempfile::TempDir;
use tokio::io::AsyncWriteExt as _;

use crate::kit::{Anima, Outcome};

/// A working repository with one commit on `main`, pushed to a bare `origin`.
pub struct Repos {
    root: TempDir,
    anima: Anima,
}

impl Repos {
    pub fn new() -> Self {
        let root = tempfile::tempdir().expect("a folder");
        for folder in ["home", "tmp", "remote.git", "work"] {
            std::fs::create_dir(root.path().join(folder)).expect("a folder");
        }
        std::fs::write(root.path().join("gitconfig"), "").expect("an empty git config");
        let anima = Anima::new()
            .with("PATH", &std::env::var("PATH").expect("PATH"))
            .with("GIT_CONFIG_NOSYSTEM", "1")
            .with(
                "GIT_CONFIG_GLOBAL",
                &path_text(&root.path().join("gitconfig")),
            )
            .with("TMPDIR", &path_text(&root.path().join("tmp")));
        let repos = Self { root, anima };
        repos.git_in(
            &repos.remote(),
            &["init", "--bare", "--quiet", "-b", "main"],
        );
        repos.git(&["init", "--quiet", "-b", "main"]);
        repos.git(&["config", "user.name", "Operator"]);
        repos.git(&["config", "user.email", "operator@example.invalid"]);
        repos.git(&["remote", "add", "origin", &path_text(&repos.remote())]);
        repos.write("a.txt", "one\n");
        repos.write("b.txt", "first\n");
        repos.write(".gitignore", "*.log\n");
        repos.git(&["add", "--all"]);
        repos.git(&["commit", "--quiet", "-m", "start"]);
        repos.git(&["push", "--quiet", "origin", "main"]);
        repos
    }

    /// The working repository.
    pub fn work(&self) -> PathBuf {
        self.root.path().join("work")
    }

    /// The bare repository `origin` points at.
    pub fn remote(&self) -> PathBuf {
        self.root.path().join("remote.git")
    }

    /// The folder the program's temporary files go to.
    pub fn temporary(&self) -> PathBuf {
        self.root.path().join("tmp")
    }

    /// An empty folder that is no repository.
    pub fn plain_folder(&self) -> PathBuf {
        self.root.path().join("home")
    }

    pub fn write(&self, path: &str, text: &str) {
        let file = self.work().join(path);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("folders");
        std::fs::write(file, text).expect("a file");
    }

    /// git in the working repository; it must succeed.
    pub fn git(&self, args: &[&str]) -> String {
        self.git_in(&self.work(), args)
    }

    /// git in `folder`; it must succeed.
    pub fn git_in(&self, folder: &Path, args: &[&str]) -> String {
        let output = self.git_command(folder, args).output().expect("git starts");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("UTF-8")
    }

    /// git in `folder`, whatever its exit status.
    pub fn git_command(&self, folder: &Path, args: &[&str]) -> std::process::Command {
        let mut command = std::process::Command::new("git");
        command
            .args(args)
            .current_dir(folder)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.root.path().join("gitconfig"))
            .env("HOME", self.root.path().join("home"))
            .env("GIT_OPTIONAL_LOCKS", "0")
            .stdin(Stdio::null());
        command
    }

    /// The scratch refs of the bare `origin`, as `<hash> <ref>` lines.
    pub fn remote_scratch_refs(&self) -> String {
        self.git_in(
            &self.remote(),
            &[
                "for-each-ref",
                "refs/anima/",
                "--format=%(objectname) %(refname)",
            ],
        )
    }

    /// A clone of `origin` that fetched `scratch`; the files of the snapshot are at `FETCH_HEAD`.
    pub fn second_machine(&self, scratch: &str) -> PathBuf {
        let second = self.root.path().join("second");
        let remote = path_text(&self.remote());
        self.git_in(self.root.path(), &["clone", "--quiet", &remote, "second"]);
        self.git_in(&second, &["fetch", "--quiet", "origin", scratch]);
        second
    }

    /// `anima <args>` in the working repository.
    pub async fn anima(&self, args: &[&str]) -> Outcome {
        self.anima_in(&self.work(), args).await
    }

    /// `anima <args>` in `folder`, standard input a pipe that is closed at once.
    pub async fn anima_in(&self, folder: &Path, args: &[&str]) -> Outcome {
        let mut command = self.anima.command(env!("CARGO_BIN_EXE_anima"));
        command
            .args(args)
            .current_dir(folder)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().expect("anima starts");
        let mut stdin = child.stdin.take().expect("a stdin pipe");
        stdin.shutdown().await.expect("stdin closed");
        drop(stdin);
        let output = child.wait_with_output().await.expect("anima ends");
        Outcome {
            code: output.status.code().expect("an exit code"),
            stdout: String::from_utf8(output.stdout).expect("UTF-8 output"),
            stderr: String::from_utf8(output.stderr).expect("UTF-8 errors"),
        }
    }

    /// Whether the repository at `folder` has the object `hash`.
    pub fn has_object(&self, folder: &Path, hash: &str) -> bool {
        self.git_command(folder, &["cat-file", "-e", hash])
            .output()
            .expect("git starts")
            .status
            .success()
    }

    /// The blob hash of `text`, from a copy outside every repository.
    pub fn blob_hash(&self, text: &str) -> String {
        let copy = self.root.path().join("outside.txt");
        std::fs::write(&copy, text).expect("a copy");
        let hash = self.git_in(self.root.path(), &["hash-object", "outside.txt"]);
        std::fs::remove_file(copy).expect("the copy removed");
        hash.trim().to_owned()
    }

    /// Everything about the working repository that `snapshot push` must leave as it was.
    pub fn state(&self) -> State {
        let git_dir = self.work().join(".git");
        State {
            index: std::fs::read(git_dir.join("index")).expect("an index"),
            config: std::fs::read(git_dir.join("config")).expect("a config"),
            objects: files_under(&git_dir.join("objects")),
            head: std::fs::read_to_string(git_dir.join("HEAD")).expect("HEAD"),
            refs: self.git(&["for-each-ref", "--format=%(objectname) %(refname)"]),
            status: self.git(&[
                "status",
                "--porcelain=v2",
                "--branch",
                "--untracked-files=all",
            ]),
            staged: self.git(&["diff", "--cached"]),
            unstaged: self.git(&["diff"]),
            stash: self.git(&["stash", "list"]),
        }
    }
}

/// The parts of a repository a snapshot must not change.
#[derive(Debug, PartialEq, Eq)]
pub struct State {
    index: Vec<u8>,
    config: Vec<u8>,
    /// Every file of the object store, so a new object shows.
    objects: Vec<PathBuf>,
    head: String,
    refs: String,
    status: String,
    staged: String,
    unstaged: String,
    stash: String,
}

/// Every file under `folder`, sorted.
fn files_under(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(folder).expect("a folder") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            found.extend(files_under(&path));
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn path_text(path: &Path) -> String {
    path.to_str().expect("a UTF-8 temporary path").to_owned()
}
