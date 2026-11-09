//! Tests of the collector: the command table (`commands`), the pure core (`outputs`, `plan`,
//! `failures`, `gpus`, `discovery`), the bounded fan-out and the deadline on paused time
//! (`fan_out`), the feed, and rounds against a test SSH device on loopback (`rounds`,
//! `gpu_rounds`).

mod bench;
mod commands;
mod discovery;
mod failures;
mod fan_out;
mod feed;
mod gpu_rounds;
mod gpus;
mod outputs;
mod plan;
mod rounds;
mod server;
