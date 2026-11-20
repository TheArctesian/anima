//! What the program would do for one item: the files that it writes and the programs that it
//! starts, each with its list of arguments (contract K10 Sections 6.4.1 and 6.4.5). A plan is
//! a value. Nothing here writes a file or starts a program.

/// The real paths that the program resolved at its start (Section 6.4.1). Each is absolute,
/// with each link resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    /// The `docker` program.
    pub docker: String,
    /// The Compose program.
    pub compose: String,
    /// The Unix socket of the engine.
    pub socket: String,
    /// The configuration folder of the program for Docker, which only the user that the
    /// program runs as can write.
    pub configuration: String,
    /// The folder of the applications that Anima installed, which only the user that the
    /// program runs as can reach (Section 6.4.5).
    pub root: String,
}

/// One program to start, directly and never through a shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// The real path of the program.
    pub program: String,
    /// The arguments, one text each. No text is read a second time by a shell.
    pub arguments: Vec<String>,
    /// The whole environment of the program: it gets no other variable.
    pub environment: Vec<(String, String)>,
    /// The working folder.
    pub folder: Option<String>,
}

/// One step of a plan. The caller does the steps in their order and stops at the first that
/// fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Makes a folder that only the user of the program can reach, when it is not there.
    Folder { path: String },
    /// Writes a file that only the user of the program can read, in place of an earlier one.
    Write { path: String, content: Vec<u8> },
    /// Moves a file in place of an earlier one, when the source is there.
    Move { from: String, to: String },
    /// Deletes a file, when it is there.
    Delete { path: String },
    /// Deletes a folder with all below it, when it is there, without following a link.
    DeleteFolder { path: String },
    /// Stops the plan when the path exists.
    Absent { path: String },
    /// Stops the plan unless `path` is a real folder directly below the real folder
    /// `below`, checked without following a link (SF-016).
    RealFolder { path: String, below: String },
    /// Starts a program and waits for its end.
    Run(Command),
}

/// The steps of one item.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub steps: Vec<Step>,
}

/// One Compose project as a command names it.
pub(crate) struct Project<'a> {
    pub name: &'a str,
    /// The working folder of Compose, which is also the folder of the project.
    pub folder: &'a str,
    pub files: &'a [String],
}

impl Paths {
    /// The environment of each command: only what the program sets (SF-041, SF-042). The
    /// variable names the configuration folder a second time, for a program that does not
    /// read the argument.
    fn environment(&self) -> Vec<(String, String)> {
        vec![("DOCKER_CONFIG".to_owned(), self.configuration.clone())]
    }

    /// The first arguments of each command: the socket and the configuration folder.
    fn first_arguments(&self) -> Vec<String> {
        vec![
            "--host".to_owned(),
            format!("unix://{}", self.socket),
            "--config".to_owned(),
            self.configuration.clone(),
        ]
    }

    /// A command of the `docker` program. `operands` come after `--`, so that none is read
    /// as an option.
    pub(crate) fn docker(&self, verbs: &[&str], operands: &[&str]) -> Command {
        let mut arguments = self.first_arguments();
        arguments.extend(verbs.iter().map(|verb| (*verb).to_owned()));
        arguments.push("--".to_owned());
        arguments.extend(operands.iter().map(|operand| (*operand).to_owned()));
        Command {
            program: self.docker.clone(),
            arguments,
            environment: self.environment(),
            folder: None,
        }
    }

    /// A command of the Compose program, started directly at its real path, with the name of
    /// the project and the path of each file as arguments.
    pub(crate) fn compose(&self, project: &Project<'_>, verbs: &[&str]) -> Command {
        let mut arguments = self.first_arguments();
        arguments.extend(["--project-name".to_owned(), project.name.to_owned()]);
        arguments.extend(["--project-directory".to_owned(), project.folder.to_owned()]);
        for file in project.files {
            arguments.extend(["--file".to_owned(), file.clone()]);
        }
        arguments.extend(verbs.iter().map(|verb| (*verb).to_owned()));
        Command {
            program: self.compose.clone(),
            arguments,
            environment: self.environment(),
            folder: Some(project.folder.to_owned()),
        }
    }

    /// The folder of one application: `<root>/<UUID>`.
    pub(crate) fn application(&self, application: &str) -> Folders {
        Folders {
            application: format!("{}/{application}", self.root),
        }
    }
}

/// The folder of one application and the names of what it holds (Section 6.4.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folders {
    application: String,
}

impl Folders {
    #[must_use]
    pub fn application(&self) -> &str {
        &self.application
    }

    /// `files/`: only the user of the program can read it, and no mount can name it.
    #[must_use]
    pub fn files(&self) -> String {
        format!("{}/files", self.application)
    }

    /// `binds/`: one folder for each bind mount.
    #[must_use]
    pub fn binds(&self) -> String {
        format!("{}/binds", self.application)
    }

    fn file(&self, name: &str) -> String {
        format!("{}/files/{name}", self.application)
    }

    /// The Compose file as it came.
    #[must_use]
    pub fn compose(&self) -> String {
        self.file("compose-as-it-came.yaml")
    }

    /// The environment file. Its name is not one that Compose reads.
    #[must_use]
    pub fn environment(&self) -> String {
        self.file("environment-of-the-application")
    }

    /// The final form, the one file that Compose reads.
    #[must_use]
    pub fn final_form(&self) -> String {
        self.file("final-form.json")
    }

    /// The image digests of the version that runs.
    #[must_use]
    pub fn digests(&self) -> String {
        self.file("digests.json")
    }

    /// Each image that the program pulled for the application.
    #[must_use]
    pub fn pulled(&self) -> String {
        self.file("pulled.json")
    }

    /// The Compose file, the environment file and the digests of the kept version.
    #[must_use]
    pub fn kept(&self) -> [String; 3] {
        [
            self.file("kept-compose-as-it-came.yaml"),
            self.file("kept-environment-of-the-application"),
            self.file("kept-digests.json"),
        ]
    }

    /// The file that Compose would read by itself in its working folder. `files/` never
    /// holds it (Section 6.4.1).
    #[must_use]
    pub fn dot_env(&self) -> String {
        self.file(".env")
    }
}
