//! Only this module calls unsafe Win32 APIs. Files outlive borrowed handles; descriptor
//! and token buffers outlive every SID/ACL pointer into them.
mod descriptor;
mod identity;
pub(crate) use descriptor::{check, copy_acl, protect};

pub mod console;
