//! The named changes GitHub can make (OD-E14, PRJ-027 to PRJ-035): how each one is sent and
//! what GitHub's reply to it means. Pure: the engine sends the request and reads the headers.

mod render;
mod settle;

pub(crate) use render::render;
pub(crate) use settle::settle;

#[cfg(test)]
mod tests;
