//! The names the hub's own device can take (HMD-045).

use crate::fleet::devices::hub_device::{free_name, names};

fn taken(names: &[&str]) -> Vec<String> {
    names.iter().map(|&name| name.to_owned()).collect()
}

#[test]
fn hmd_045_the_hub_takes_the_name_hub_when_it_is_free() {
    assert_eq!(free_name(names(), &[]).as_deref(), Some("Hub"));
    let other = taken(&["hub", "Hub (hosted)"]);
    assert_eq!(free_name(names(), &other).as_deref(), Some("Hub"));
}

#[test]
fn hmd_045_a_taken_name_gives_the_next_one() {
    let one = taken(&["Hub"]);
    assert_eq!(free_name(names(), &one).as_deref(), Some("Hub (hosted)"));
    let two = taken(&["Hub (hosted)", "Hub"]);
    assert_eq!(free_name(names(), &two).as_deref(), Some("Hub (hosted 2)"));
}

#[test]
fn hmd_045_no_name_is_given_when_each_one_is_taken() {
    assert_eq!(free_name(names(), &names()), None);
}
