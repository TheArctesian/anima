//! Unit tests of the registry data and the pure switch plan. Behaviour against the database and
//! GraphQL is tested in `tests/it/extensions`.

use std::collections::HashSet;

use proptest::prelude::*;

use super::{
    Entry, Key, PROJECTS, on_set, registry,
    switching::{Refusal, plan},
};

fn key(text: &str) -> Key {
    Key::parse(text).unwrap()
}

fn on(keys: &[&str]) -> HashSet<Key> {
    on_set(registry(), keys.iter().copied())
}

#[test]
fn ext_001_the_registry_lists_fifteen_and_only_projects_and_the_engine_are_built() {
    assert_eq!(registry().len(), 15);
    let built: Vec<_> = registry()
        .iter()
        .filter(|entry| entry.built)
        .map(|entry| entry.key.as_str())
        .collect();
    assert_eq!(built, ["projects", "integrations"]);
    assert_eq!(PROJECTS.as_str(), "projects");
}

#[test]
fn ext_010_no_needed_entry_has_needs() {
    for entry in registry() {
        for need in entry.needs {
            assert!(
                need.entry().needs.is_empty(),
                "{} needs {}",
                entry.name,
                need.name()
            );
        }
    }
}

#[test]
fn ext_001_a_stored_key_outside_the_registry_is_ignored_and_the_fixed_one_is_always_on() {
    let now = on(&["gone", "projects"]);
    assert_eq!(now, HashSet::from([key("projects"), key("integrations")]));
}

#[test]
fn ext_011_on_from_empty_names_the_extension_then_each_unbuilt_switch_it_needs() {
    assert_eq!(
        plan(registry(), &on(&[]), PROJECTS, true),
        Ok(vec![PROJECTS])
    );
    assert_eq!(
        plan(registry(), &on(&["projects"]), PROJECTS, true),
        Ok(vec![])
    );
}

#[test]
fn ext_011_an_unbuilt_extension_and_the_fixed_one_are_refused() {
    let unbuilt = key("map");
    assert_eq!(
        plan(registry(), &on(&[]), unbuilt, true),
        Err(Refusal::NotBuilt(unbuilt))
    );
    assert_eq!(
        plan(registry(), &on(&[]), unbuilt, false),
        Err(Refusal::NotBuilt(unbuilt))
    );
    let fixed = key("integrations");
    assert_eq!(
        plan(registry(), &on(&[]), fixed, false),
        Err(Refusal::Fixed(fixed))
    );
}

#[test]
fn ext_003_off_changes_only_a_switch_that_is_on() {
    assert_eq!(
        plan(registry(), &on(&["projects"]), PROJECTS, false),
        Ok(vec![PROJECTS])
    );
    assert_eq!(plan(registry(), &on(&[]), PROJECTS, false), Ok(vec![]));
}

#[test]
fn ext_012_the_refusal_names_the_dependant_in_the_operators_words() {
    let refusal = Refusal::Needed {
        key: key("secrets"),
        by: vec![key("model_gateway")],
    };
    assert_eq!(
        refusal.to_string(),
        "Model gateway is on and needs Secrets. Set Model gateway off first."
    );
}

/// Applies a plan to the set of keys that are on, as the mutation does.
fn apply(mut on_now: HashSet<Key>, changed: &[Key], want: bool) -> HashSet<Key> {
    for key in changed {
        if want {
            on_now.insert(*key);
        } else {
            on_now.remove(key);
        }
    }
    on_now
}

/// The registry with every extension built, so the property covers all fifteen.
fn all_built() -> Vec<Entry> {
    registry()
        .iter()
        .map(|e| Entry {
            key: e.key,
            name: e.name,
            data_class: e.data_class,
            needs: e.needs,
            works_with: e.works_with,
            navigation: e.navigation,
            migrations: e.migrations,
            built: true,
            fixed: e.fixed,
            setting: e.setting,
        })
        .collect()
}

fn consistent(on_now: &HashSet<Key>) -> bool {
    registry()
        .iter()
        .filter(|entry| on_now.contains(&entry.key))
        .all(|entry| entry.needs.iter().all(|need| on_now.contains(need)))
}

proptest! {
    #[test]
    fn ext_011_ext_012_no_plan_leaves_a_needed_extension_off_while_its_dependant_is_on(
        steps in prop::collection::vec((0..registry().len(), any::<bool>()), 0..40)
    ) {
        let built = all_built();
        let mut on_now = on(&[]);
        for (index, want) in steps {
            let key = built[index].key;
            if let Ok(changed) = plan(&built, &on_now, key, want) {
                on_now = apply(on_now, &changed, want);
            }
            prop_assert!(consistent(&on_now));
        }
    }

    #[test]
    fn ext_012_from_any_consistent_start_every_request_keeps_the_rule(
        start in prop::collection::vec(any::<bool>(), 15), index in 0..15usize, want in any::<bool>()
    ) {
        let on_now: HashSet<Key> = registry().iter().zip(start)
            .filter(|(entry, flag)| entry.fixed || *flag).map(|(entry, _)| entry.key).collect();
        prop_assume!(consistent(&on_now));
        let built = all_built();
        if let Ok(changed) = plan(&built, &on_now, built[index].key, want) {
            prop_assert!(consistent(&apply(on_now, &changed, want)));
        }
    }
}

// The write path against the database, over a registry in which every extension is built so the
// needs of Model gateway can be exercised.

mod writing {
    use chrono::{TimeZone, Utc};
    use sqlx::PgPool;

    use super::{Key, all_built, key};
    use crate::engine::extensions::{Error, Refusal, on_keys, store, switch};

    async fn set(pool: &PgPool, key: Key, want: bool) -> Result<Vec<Key>, Error> {
        let at = Utc.with_ymd_and_hms(2030, 5, 5, 12, 0, 0).unwrap();
        let mut tx = pool.begin().await.unwrap();
        let changed = switch(&mut tx, &all_built(), (key, want), (at, "operator")).await?;
        tx.commit().await.unwrap();
        Ok(changed)
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn ext_011_on_from_empty_sets_each_need_on_and_returns_what_changed(pool: PgPool) {
        let changed = set(&pool, key("model_gateway"), true).await.unwrap();
        assert_eq!(changed, [key("model_gateway"), key("secrets")]);
        let on = on_keys(&pool).await.unwrap();
        assert!(on.contains(&key("secrets")) && on.contains(&key("model_gateway")));
        assert_eq!(set(&pool, key("model_gateway"), true).await.unwrap(), []);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn ext_012_off_while_needed_is_refused_naming_the_dependant_and_changes_nothing(
        pool: PgPool,
    ) {
        set(&pool, key("model_gateway"), true).await.unwrap();
        let before = store::switched_on(&pool).await.unwrap();
        let refused = set(&pool, key("secrets"), false).await.unwrap_err();
        let Error::Refused(Refusal::Needed { by, .. }) = &refused else {
            panic!("{refused:?}");
        };
        assert_eq!(by, &[key("model_gateway")]);
        assert_eq!(store::switched_on(&pool).await.unwrap(), before);
        set(&pool, key("model_gateway"), false).await.unwrap();
        assert_eq!(
            set(&pool, key("secrets"), false).await.unwrap(),
            [key("secrets")]
        );
    }
}
