//! Only this module calls unsafe Win32 APIs. Files outlive borrowed handles; descriptor
//! and token buffers outlive every SID/ACL pointer into them.
mod descriptor;
mod identity;
pub(crate) mod rename;
pub(crate) use descriptor::{check, copy_acl, protect, single_name};

pub mod console;

pub(crate) mod environment;

pub mod process;

pub mod job;

pub mod socket;

pub mod power;
