//! Tests of the collector: the command table (`commands`), the pure core (`outputs`, `plan`,
//! `failures`, `os_release`), the bounded fan-out and the deadline on paused time (`fan_out`), the feed, and rounds against a test SSH device on loopback (`rounds`).

mod bench;
mod commands;
mod failures;
mod fan_out;
mod feed;
mod os_release;
mod outputs;
mod plan;
mod rounds;
mod server;
