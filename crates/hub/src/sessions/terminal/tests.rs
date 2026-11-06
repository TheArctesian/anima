//! Tests of the terminal (K4 Section 7): the text-message parser, property-tested, the
//! hub's waits and SSH settings (`limits`), the hand-off of a PTY or shell request
//! (`requests`), and the terminal over real sockets (`wire`).

mod control;
mod limits;
mod requests;
mod wire;
