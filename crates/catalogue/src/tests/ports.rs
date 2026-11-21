//! The ports of Anima on a machine (SF-NEW-revlsform): no recipe publishes one of them.

use super::{codes, sound, with_compose};
use crate::{ANIMA_PORTS, Code};

#[test]
fn sf_revlsform_a_recipe_that_publishes_a_port_of_anima_is_refused() {
    assert_eq!(codes(&sound()), []);
    for anima in ANIMA_PORTS {
        let published = with_compose("published: 8080", &format!("published: {anima}"));
        assert_eq!(codes(&published), [Code::AnimaPortPublished], "{anima}");
    }
}
