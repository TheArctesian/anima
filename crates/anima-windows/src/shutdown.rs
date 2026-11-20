//! Native shutdown requests: console signals or a managed parent's private stdin pipe.
pub struct Requested {
    interrupt: tokio::signal::windows::CtrlC,
    terminate: tokio::signal::windows::CtrlBreak,
    input: tokio::sync::mpsc::Receiver<()>,
    _keep_input: tokio::sync::mpsc::Sender<()>,
}

impl Requested {
    /// Buffer shutdown requests before startup begins. EOF alone is not a request;
    /// an owned job is responsible for ending children when the parent crashes.
    /// # Errors
    /// Windows cannot install the console signal listeners.
    pub fn new() -> std::io::Result<Self> {
        let interrupt = tokio::signal::windows::ctrl_c()?;
        let terminate = tokio::signal::windows::ctrl_break()?;
        let (keep, input) = tokio::sync::mpsc::channel(1);
        let sender = keep.clone();
        std::thread::spawn(move || {
            use std::io::{BufRead, Read};
            let mut line = String::new();
            if std::io::stdin()
                .lock()
                .take(16)
                .read_line(&mut line)
                .is_ok()
                && line.trim() == "stop"
            {
                let _ = sender.blocking_send(());
            }
        });
        Ok(Self {
            interrupt,
            terminate,
            input,
            _keep_input: keep,
        })
    }

    /// Wait for one shutdown request. The first request sent before this call is retained.
    pub async fn wait(&mut self) {
        tokio::select! {
            _ = self.interrupt.recv() => {},
            _ = self.terminate.recv() => {},
            _ = self.input.recv() => {},
        }
    }
}
