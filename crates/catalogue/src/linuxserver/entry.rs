//! The entry file of a generated recipe, in the order of the fields of Section 13.2: the shape
//! that `serde_json` writes. [`crate::entry::Entry`] reads it back.

use serde::Serialize;

use crate::entry::{MachineTest, Reviewed};

#[derive(Serialize)]
pub(super) struct Written<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: String,
    pub version: &'a str,
    pub author: &'static str,
    pub tags: Vec<String>,
    pub compose_url: &'static str,
    pub env_url: &'static str,
    pub documentation_url: &'a str,
    pub icon_url: Option<&'a str>,
    pub anima: Anima<'a>,
}

#[derive(Serialize)]
pub(super) struct Anima<'a> {
    pub class: &'static str,
    pub origin: Origin<'a>,
    pub reviewed: &'a Reviewed,
    pub images: [Pinned<'a>; 1],
    pub marks: [(); 0],
    pub exceptions: [Exception<'a>; 1],
    pub streams: Vec<Stream>,
    pub optional: Vec<Device<'a>>,
    pub volumes: Vec<Kept<'a>>,
    pub generated: Vec<&'a str>,
    pub test: Test<'a>,
    pub pinned: &'a str,
    pub last_test: MachineTest,
}

#[derive(Serialize)]
pub(super) struct Origin<'a> {
    pub from: &'static str,
    pub read: &'a str,
}

#[derive(Serialize)]
pub(super) struct Pinned<'a> {
    pub service: &'a str,
    pub image: String,
    pub tag: &'a str,
    pub resolved: &'a str,
}

#[derive(Serialize)]
pub(super) struct Exception<'a> {
    pub exception: &'static str,
    pub services: [&'a str; 1],
}

#[derive(Serialize)]
pub(super) struct Stream {
    pub port: u16,
    pub protocols: Vec<&'static str>,
}

#[derive(Serialize)]
pub(super) struct Device<'a> {
    pub part: &'static str,
    pub path: &'a str,
}

#[derive(Serialize)]
pub(super) struct Kept<'a> {
    pub volume: &'a str,
    pub copy: &'static str,
}

#[derive(Serialize)]
pub(super) struct Test<'a> {
    pub service: &'a str,
    pub port: u16,
    pub reach: &'static str,
    pub path: Option<&'static str>,
    pub statuses: [u16; 0],
    pub body: Option<&'static str>,
    pub limit_seconds: u32,
}
