//! One `anima mcp` process and the client side of its JSON-RPC session. Every line it prints
//! on standard output is kept, so a test can check that all of them are JSON-RPC.

use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::kit::Anima;
use crate::kit::fake_hub::{FakeHub, data};

pub const LOGIN_TOKEN: &str = "login-token-for-mcp-tests";
pub const API_KEY: &str = "anima_mcpTestKeyAbcdefghijklmnopqrstuvwxyz0123456";

/// The operator's computer: a home folder for the configuration file.
pub struct Desk {
    anima: Anima,
    /// What the MCP client declares at `initialize`.
    capabilities: Value,
}

pub struct Session {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
    /// Every line read from standard output.
    pub printed: Vec<String>,
    next_id: u64,
    /// The result sent back for each `elicitation/create` request; `None` answers with a
    /// JSON-RPC error.
    pub approval: Option<Value>,
    /// The `params` of each `elicitation/create` request the server sent.
    pub asked: Vec<Value>,
}

/// A hub that accepts the token at start (`me`).
pub async fn hub() -> FakeHub {
    let hub = FakeHub::start().await;
    hub.answer(
        "Me",
        data(
            json!({ "me": { "id": "0190a000-0000-7000-8000-000000000001",
                             "username": "operator", "totpEnabled": true } }),
        ),
    );
    hub
}

impl Desk {
    pub fn new() -> Self {
        Self::on(Anima::new())
    }

    /// The desk of `anima`'s computer: the same home folder and variables.
    pub fn on(anima: Anima) -> Self {
        Self {
            anima,
            capabilities: json!({}),
        }
    }

    /// The command line of this computer.
    pub const fn anima(&self) -> &Anima {
        &self.anima
    }

    /// A desk whose MCP client can ask the person (form elicitation, MCP-013).
    pub fn asking() -> Self {
        Self::new().with_elicitation()
    }

    /// This desk with an MCP client that can ask the person (form elicitation, MCP-013).
    pub fn with_elicitation(self) -> Self {
        Self {
            capabilities: json!({ "elicitation": { "form": {} } }),
            ..self
        }
    }

    /// Writes `saved` as the MCP permissions of `origin` in the configuration file, as the
    /// operator could by hand. The tests use it for permissions that widen access:
    /// `anima mcp permissions set` makes those only at a terminal (MCP-014), and the tests run
    /// it without one. Process-level tests keep the command for showing, narrowing and refusal.
    pub fn save_permissions(&self, origin: &str, saved: &Value) {
        let dir = self.anima.config_dir();
        std::fs::create_dir_all(&dir).expect("the configuration folder");
        let config = json!({ "contexts": {}, "mcp": { origin: saved } });
        std::fs::write(dir.join("config.json"), config.to_string()).expect("written");
    }

    fn command(&self, token: &str, args: &[&str]) -> Command {
        let mut command = self.anima.command(env!("CARGO_BIN_EXE_anima"));
        command
            .args(args)
            .env("ANIMA_TOKEN", token)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        command
    }

    /// Runs `anima` to the end with no input; returns the exit code, stdout and stderr.
    pub async fn run(&self, token: &str, args: &[&str]) -> (i32, String, String) {
        let output = self
            .command(token, args)
            .stdin(Stdio::null())
            .output()
            .await
            .expect("anima runs");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8(output.stdout).expect("UTF-8"),
            String::from_utf8(output.stderr).expect("UTF-8"),
        )
    }

    /// Starts `anima mcp <args> --hub <hub>` and initialises the session.
    pub async fn start(&self, hub: &FakeHub, token: &str, args: &[&str]) -> (Session, Value) {
        self.start_at(&hub.origin, token, args).await
    }

    /// Starts `anima mcp <args> --hub <origin>` and initialises the session.
    pub async fn start_at(&self, origin: &str, token: &str, args: &[&str]) -> (Session, Value) {
        let mut all = vec!["mcp"];
        all.extend_from_slice(args);
        all.extend_from_slice(&["--hub", origin]);
        let mut child = self.command(token, &all).spawn().expect("anima mcp starts");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout")).lines();
        let mut session = Session {
            child,
            stdin,
            stdout,
            printed: Vec::new(),
            next_id: 1,
            approval: None,
            asked: Vec::new(),
        };
        let initialised = session
            .request(
                "initialize",
                json!({ "protocolVersion": "2025-06-18", "capabilities": self.capabilities,
                        "clientInfo": { "name": "it", "version": "1" } }),
            )
            .await;
        session
            .send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .await;
        (session, initialised)
    }
}

impl Session {
    async fn send(&mut self, message: &Value) {
        let mut line = message.to_string();
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await.expect("sent");
        self.stdin.flush().await.expect("flushed");
    }

    /// Sends a request and returns its `result` (or the whole answer when it has none).
    pub async fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .await;
        loop {
            let line = tokio::time::timeout(Duration::from_secs(30), self.stdout.next_line())
                .await
                .expect("an answer in time")
                .expect("readable")
                .expect("a line before the end");
            self.printed.push(line.clone());
            let answer: Value = serde_json::from_str(&line).expect("each line is JSON");
            if answer["method"] == "elicitation/create" {
                self.answer_elicitation(&answer).await;
            } else if answer["id"] == json!(id) {
                return answer.get("result").cloned().unwrap_or(answer);
            }
        }
    }

    /// Answers the server's `elicitation/create` request with [`Session::approval`].
    async fn answer_elicitation(&mut self, request: &Value) {
        self.asked.push(request["params"].clone());
        let reply = match &self.approval {
            Some(result) => json!({ "jsonrpc": "2.0", "id": request["id"], "result": result }),
            None => json!({ "jsonrpc": "2.0", "id": request["id"],
                            "error": { "code": -32603, "message": "no person here" } }),
        };
        self.send(&reply).await;
    }

    /// The names of the tools offered.
    pub async fn tool_names(&mut self) -> Vec<String> {
        let listed = self.request("tools/list", json!({})).await;
        listed["tools"]
            .as_array()
            .expect("a tool list")
            .iter()
            .map(|tool| tool["name"].as_str().unwrap_or_default().to_owned())
            .collect()
    }

    /// Calls `name` with `arguments` and returns the tool result.
    pub async fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
        .await
    }

    /// Closes standard input and returns what the server printed on standard error.
    pub async fn finish(mut self) -> (Vec<String>, String) {
        drop(self.stdin);
        while let Ok(Some(line)) = self.stdout.next_line().await {
            self.printed.push(line);
        }
        let output = tokio::time::timeout(Duration::from_secs(30), self.child.wait_with_output())
            .await
            .expect("anima mcp ends when its input ends")
            .expect("an exit");
        (
            self.printed,
            String::from_utf8(output.stderr).expect("UTF-8"),
        )
    }
}

/// Fails unless every line is a JSON-RPC 2.0 message and none holds `secret`.
pub fn assert_protocol_only(printed: &[String], stderr: &str, secret: &str) {
    for line in printed {
        let message: Value = serde_json::from_str(line).expect("JSON");
        assert_eq!(message["jsonrpc"], "2.0", "{line}");
        assert!(!line.contains(secret), "token on stdout: {line}");
    }
    assert!(!stderr.contains(secret), "token on stderr: {stderr}");
}
