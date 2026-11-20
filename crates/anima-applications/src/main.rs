//! The applications program (contract K10 Section 6.4.1). It is not complete: the library has
//! the checks and the plans, and no part that opens the socket or starts a program of Docker
//! exists (task H31, part 2). Thus this program does nothing and ends with a failure.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("anima-applications: this program is not complete. It does nothing.");
    ExitCode::FAILURE
}
