//! The debug text of each value that holds the two files or the final form (K10 Section
//! 6.4.5: only the user of the program can read them).

use super::{COMPOSE, ENVIRONMENT, digests, plan, prepare};
use crate::{
    tests::World,
    work::{
        apply::{Files, final_form_of},
        back::Kept,
    },
};

#[test]
fn k10_6_4_5_a_value_that_holds_the_files_shows_no_secret_value_in_its_debug_text() {
    let world = World::new();
    let prepared = prepare(&world, COMPOSE).expect("the files pass");
    let planned = plan(&world, true);
    let files = Files {
        compose: COMPOSE.as_bytes(),
        environment: Some(ENVIRONMENT.as_bytes()),
    };
    let kept = Kept {
        compose: COMPOSE.as_bytes(),
        environment: ENVIRONMENT.as_bytes(),
    };
    let form = final_form_of(&prepared, &digests(), &world.facts()).expect("a final form");
    let texts = [
        format!("{prepared:?}"),
        format!("{planned:?}"),
        format!("{:?}", planned.plan),
        format!("{files:?}"),
        format!("{kept:?}"),
        format!("{form:?}"),
        format!("{:?}", prepared.checked),
    ];
    // The secret value of the environment file is `s3cr$t`; the final form writes `s3cr$$t`.
    // A list of bytes would show `s3cr` as `115, 51, 99, 114`.
    for text in texts {
        assert!(!text.contains("s3cr"), "{text}");
        assert!(!text.contains("TOKEN"), "{text}");
        assert!(!text.contains("115, 51, 99, 114"), "{text}");
    }
    // The length of each written file is shown.
    assert!(format!("{:?}", planned.plan).contains("content_bytes"));
}
