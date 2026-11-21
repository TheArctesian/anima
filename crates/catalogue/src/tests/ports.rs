//! The ports of a machine (SF-252): no recipe publishes a port of Anima, and no two
//! recipes publish the same port, so that each two entries can run on one machine.

use super::{codes, sound, with_compose};
use std::collections::BTreeMap;

use crate::{ANIMA_PORTS, Code, published, shipped};

#[test]
fn sf_revlsform_a_recipe_that_publishes_a_port_of_anima_is_refused() {
    assert_eq!(codes(&sound()), []);
    for anima in ANIMA_PORTS {
        let published = with_compose("published: 8080", &format!("published: {anima}"));
        assert_eq!(codes(&published), [Code::AnimaPortPublished], "{anima}");
    }
}

#[test]
fn sf_revlsform_no_two_shipped_recipes_publish_the_same_port_of_the_machine() {
    let mut owners: BTreeMap<u16, Vec<String>> = BTreeMap::new();
    for recipe in shipped() {
        for port in published(&recipe) {
            owners.entry(port).or_default().push(recipe.folder.clone());
        }
    }
    assert!(owners.len() > 10, "{owners:?}");
    let shared: Vec<_> = owners
        .iter()
        .filter(|(_, folders)| folders.len() > 1)
        .collect();
    assert_eq!(shared, Vec::<(&u16, &Vec<String>)>::new());
}
