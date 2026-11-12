//! Where the agent goes: only through folders no other user can change (T6a's rule, with the
//! group, an ACL and links), and to the real path that was checked, which is the one run.

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

use super::rig::{Rig, rig};
use crate::agent::{AgentError, trust::User};

#[test]
fn met_019_the_agent_runs_from_the_real_path_that_was_checked() {
    let rig = rig("exit 0");
    let real = rig.dir.path().join("real-bin");
    fs::create_dir(&real).expect("the real folder");
    fs::create_dir(rig.home.join(".local")).expect(".local");
    symlink(&real, rig.home.join(".local/bin")).expect("a link");
    assert_eq!(rig.install(), Ok(real.join("anima-agent")));
    let program = format!("program:{}\n", real.join("anima-agent").display());
    assert_eq!(rig.runs().matches(&program).count(), 2, "{}", rig.runs());
}

#[test]
fn met_019_a_folder_others_can_write_is_refused_before_anything_runs() {
    for open in [".local", ".local/bin", ""] {
        let rig = rig("exit 0");
        let folder = rig.home.join(open);
        fs::create_dir_all(&folder).expect("the folder");
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o777)).expect("chmod");
        assert_eq!(rig.install(), Err(AgentError::Untrusted), "{open}");
        assert!(!rig.installed().exists(), "{open}");
        assert_eq!(rig.runs(), "", "{open}");
        if open != ".local/bin" {
            // Checked before anything is made: no folder goes inside an untrusted one.
            assert!(!rig.home.join(".local/bin").exists(), "{open}");
        }
    }
}

#[test]
fn met_019_a_folder_a_shared_group_can_write_is_refused_and_the_private_group_is_not() {
    let rig = rig("exit 0");
    let bin = rig.local_bin();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o775)).expect("chmod");
    assert_eq!(
        rig.install(),
        Ok(rig.installed()),
        "the user's private group"
    );
    // The same folder when its group is not the user's private one (`chgrp adm`, 0775).
    let shared = Rig {
        user: User {
            private_group: None,
            ..rig.user
        },
        ..rig
    };
    fs::remove_file(&shared.record).expect("the record");
    assert_eq!(shared.install(), Err(AgentError::Untrusted));
    assert_eq!(shared.runs(), "");
}

/// `setfacl -m u:nobody:rwx` on `folder`, written as the kernel's ACL attribute (version 2, then
/// tag, permissions and ID per entry), so no `setfacl` is needed.
#[cfg(target_os = "linux")]
fn grant_nobody_write(folder: &std::path::Path) {
    let mut acl = 2_u32.to_le_bytes().to_vec();
    let undefined = u32::MAX;
    for (tag, perm, id) in [
        (0x01_u16, 7_u16, undefined),
        (0x02, 7, 65_534),
        (0x04, 5, undefined),
        (0x10, 7, undefined),
        (0x20, 5, undefined),
    ] {
        acl.extend_from_slice(&tag.to_le_bytes());
        acl.extend_from_slice(&perm.to_le_bytes());
        acl.extend_from_slice(&id.to_le_bytes());
    }
    rustix::fs::setxattr(
        folder,
        "system.posix_acl_access",
        &acl,
        rustix::fs::XattrFlags::empty(),
    )
    .expect("the ACL (the file system must support POSIX ACLs)");
}

#[cfg(target_os = "linux")]
#[test]
fn met_019_a_folder_another_user_may_write_through_an_acl_is_refused() {
    let rig = rig("exit 0");
    let bin = rig.local_bin();
    grant_nobody_write(&bin);
    assert_eq!(
        std::os::unix::fs::MetadataExt::mode(&fs::metadata(&bin).expect("bin")) & 0o070,
        0o070,
        "mask rwx"
    );
    assert_eq!(rig.install(), Err(AgentError::Untrusted));
    assert_eq!(rig.runs(), "", "the code reached the agent");
}

#[test]
fn met_019_a_missing_or_linked_agent_is_not_bundled() {
    let rig = rig("exit 0");
    let moved = rig.dir.path().join("elsewhere");
    fs::rename(&rig.source, &moved).expect("moved");
    assert_eq!(rig.install(), Err(AgentError::NotBundled));
    symlink(&moved, &rig.source).expect("a link");
    assert_eq!(rig.install(), Err(AgentError::NotBundled));
    assert_eq!(rig.runs(), "");
}

#[test]
fn met_019_a_link_at_the_destination_is_replaced_not_followed() {
    let rig = rig("exit 0");
    let victim = rig.dir.path().join("victim");
    fs::write(&victim, b"keep").expect("the victim");
    rig.local_bin();
    symlink(&victim, rig.installed()).expect("a link");
    assert_eq!(rig.install(), Ok(rig.installed()));
    assert_eq!(fs::read(&victim).ok(), Some(b"keep".to_vec()));
    let installed = fs::symlink_metadata(rig.installed()).expect("installed");
    assert!(installed.is_file(), "the link was followed");
}
