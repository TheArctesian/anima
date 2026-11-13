//! One `anima mcp` process and the client side of its JSON-RPC session. Every line it prints
//! on standard output is kept, so a test can check that all of them are JSON-RPC.

use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::kit::fake_hub::{FakeHub, data};

pub const LOGIN_TOKEN: &str = "login-token-for-mcp-tests";
pub const API_KEY: &str = "anima_mcpTestKeyAbcdefghijklmnopqrstuvwxyz0123456";

/// The operator's computer: a home folder for the configuration file.
pub struct Desk {
    home: TempDir,
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
        Self {
            home: tempfile::tempdir().expect("a home folder"),
            capabilities: json!({}),
        }
    }

    /// A desk whose MCP client can ask the person (form elicitation, MCP-013).
    pub fn asking() -> Self {
        Self {
            capabilities: json!({ "elicitation": { "form": {} } }),
            ..Self::new()
        }
    }

    fn command(&self, token: &str, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_anima"));
        command
            .args(args)
            .env_clear()
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path().join("config"))
            .env("ANIMA_TOKEN_STORE", "file")
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
        let mut all = vec!["mcp"];
        all.extend_from_slice(args);
        all.extend_from_slice(&["--hub", &hub.origin]);
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
