//! What the tests of model work share: an archive in memory, the folders, the node.

use std::io::Write as _;
use std::path::Path;
use std::time::Instant;

use anima_daemon::model::cycle::Node;
use anima_daemon::model::supervise::Servers;
use daemon_core::model_work::folders::Folders;
use daemon_core::model_work::machine::MachineSettings;
use daemon_core::model_work::names::{Backend, Runtime};
use daemon_core::model_work::releases::{Release, SourceEntry, Sources};
use daemon_core::wire::{WorkArchitecture, WorkOs};
use uuid::Uuid;

/// The scripted runtime. It reads `mode` beside it: `exit` ends at once, `any` listens on
/// 0.0.0.0, else it listens on 127.0.0.1. It writes the names of its environment to `env`, and
/// before it listens it starts a child that only sleeps and writes its process id to `child`.
pub const FAKE_RUNTIME: &str = r#"#!/usr/bin/perl
use strict;
use IO::Socket::INET;
use File::Basename;
my $dir = dirname($0);
my $port = 0;
for my $i (0 .. $#ARGV - 1) { $port = $ARGV[$i + 1] if $ARGV[$i] eq '--port'; }
open(my $m, '<', "$dir/mode"); my $mode = <$m> // 'loopback'; close($m); chomp $mode;
open(my $e, '>', "$dir/env"); print $e join("\n", sort keys %ENV); close($e);
exit 1 if $mode eq 'exit';
my $kid = fork();
if (defined $kid && $kid == 0) { sleep 100000; exit 0; }
open(my $k, '>', "$dir/child"); print $k $kid; close($k);
my $host = $mode eq 'any' ? '0.0.0.0' : '127.0.0.1';
my $s = IO::Socket::INET->new(LocalAddr => $host, LocalPort => $port, Listen => 5, ReuseAddr => 1) or exit 2;
while (1) { my $c = $s->accept(); close($c) if $c; }
"#;

pub const PROGRAM: &str = "bin/llama-server";

/// A `.tar.gz` of `entries`: (path, mode, bytes), or a link when the bytes are `None`.
/// One entry of a test archive: its path, its mode, and its bytes (`None` for a link).
pub type Entry<'a> = (&'a str, u32, Option<&'a [u8]>);

pub fn archive(entries: &[Entry<'_>]) -> std::fs::File {
    let mut file = tempfile::tempfile().expect("a file");
    {
        let gz = flate2::write::GzEncoder::new(&mut file, flate2::Compression::fast());
        let mut tar = tar::Builder::new(gz);
        for (path, mode, bytes) in entries {
            // The name is written as raw bytes, so a test can make a name that the builder
            // of the `tar` crate would refuse.
            let mut header = tar::Header::new_old();
            header.as_old_mut().name[..path.len()].copy_from_slice(path.as_bytes());
            header.set_mode(*mode);
            let data = bytes.unwrap_or_default();
            header.set_size(data.len() as u64);
            if bytes.is_some() {
                header.set_entry_type(tar::EntryType::Regular);
            } else {
                header.set_entry_type(tar::EntryType::Symlink);
                header.set_link_name("/etc/passwd").expect("a link name");
            }
            header.set_cksum();
            tar.append(&header, data).expect("an entry");
        }
        tar.into_inner()
            .expect("tar")
            .finish()
            .expect("gz")
            .flush()
            .expect("flush");
    }
    std::io::Seek::rewind(&mut file).expect("rewind");
    file
}

pub fn folders(root: &Path) -> Folders {
    Folders {
        runtimes: root.join("runtimes"),
        models: root.join("models"),
        state: root.join("model-state"),
    }
}

/// What a test unpack writes, with the reserve `reserve`.
pub fn target(folders: &Folders, reserve: u64) -> anima_daemon::model::install::Target {
    anima_daemon::model::install::Target {
        folders: folders.clone(),
        release: release(),
        entry: entry(),
        reserve,
    }
}

pub fn release() -> Release {
    Release {
        runtime: Runtime::LlamaServer,
        version: "b8100".to_owned(),
        backend: Backend::Cpu,
    }
}

pub const OS: (WorkOs, &str) = if cfg!(target_os = "macos") {
    (WorkOs::Macos, "macos")
} else {
    (WorkOs::Linux, "linux")
};
pub const ARCH: (WorkArchitecture, &str) = if cfg!(target_arch = "aarch64") {
    (WorkArchitecture::Aarch64, "aarch64")
} else {
    (WorkArchitecture::X86_64, "x86_64")
};

pub fn entry() -> SourceEntry {
    SourceEntry {
        runtime: "llama_server".to_owned(),
        version: "b8100".to_owned(),
        os: OS.1.to_owned(),
        architecture: ARCH.1.to_owned(),
        backend: "cpu".to_owned(),
        address: "https://github.com/example/none.tar.gz".to_owned(),
        sha256: "0".repeat(64),
        size_bytes: 1 << 20,
        program: PROGRAM.to_owned(),
    }
}

/// A node whose servers take a port of `ports`; each test that starts a server has its own
/// range, as the tests run in parallel processes.
pub fn node(root: &Path) -> Node {
    node_on(root, 41_000..=41_999)
}

pub fn node_on(root: &Path, ports: std::ops::RangeInclusive<u16>) -> Node {
    Node {
        device: Uuid::nil(),
        os: OS.0,
        architecture: ARCH.0,
        folders: folders(root),
        sources: Sources {
            sources: vec![entry()],
        },
        settings: MachineSettings::default(),
        servers: Servers::default(),
        switch_on: true,
        started: Instant::now(),
        reserve: 0,
        ports,
    }
}
