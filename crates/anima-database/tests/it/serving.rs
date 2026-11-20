//! Exercise the supervisor's platform shutdown contract and reap it even after a failure.
use std::io::{self, BufReader};
use std::process::{Child, ChildStdout, Output};

#[derive(Clone, Copy)]
pub enum Stop {
    Terminate,
    Interrupt,
    Kill,
}

pub struct Serving {
    child: Option<Child>,
    _output: BufReader<ChildStdout>,
}

impl Serving {
    pub fn new(child: Child, output: BufReader<ChildStdout>) -> Self {
        Self {
            child: Some(child),
            _output: output,
        }
    }

    pub fn finish(mut self, stop: Stop) -> Output {
        let mut child = self.child.take().unwrap();
        end(&mut child, stop).unwrap();
        child.wait_with_output().unwrap()
    }
}

impl Drop for Serving {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if end(&mut child, Stop::Terminate).is_err() {
                child.kill().ok();
            }
            child.wait().ok();
        }
    }
}

#[cfg(unix)]
fn end(child: &mut Child, stop: Stop) -> io::Result<()> {
    use nix::{
        sys::signal::{Signal, kill},
        unistd::Pid,
    };
    let signal = match stop {
        Stop::Terminate => Signal::SIGTERM,
        Stop::Interrupt => Signal::SIGINT,
        Stop::Kill => Signal::SIGKILL,
    };
    kill(Pid::from_raw(i32::try_from(child.id()).unwrap()), signal).map_err(io::Error::from)
}

#[cfg(windows)]
fn end(child: &mut Child, stop: Stop) -> io::Result<()> {
    use std::io::Write;
    match stop {
        Stop::Kill => child.kill(),
        Stop::Terminate | Stop::Interrupt => child.stdin.as_mut().unwrap().write_all(b"stop\n"),
    }
}
