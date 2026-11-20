//! The checks at the start (K10 Section 6.4.1).

use super::World;
use crate::{
    EngineKind,
    start::{
        Entry, LOWEST_COMPOSE, LOWEST_ENGINE, StartFacts, StartRefusal, Version, decided,
        socket_owner_is_the_same,
    },
};

fn of_root() -> Entry {
    Entry {
        owner: 0,
        mode: 0o755,
        group_members: Vec::new(),
        listed_writers: Vec::new(),
        listed_for_all: false,
    }
}

/// A system engine: the program runs as the identifier 0.
fn system() -> StartFacts {
    StartFacts {
        running_as: 0,
        administrators: vec![501],
        socket_is_a_socket: true,
        socket_owner: 0,
        accepted_user_can_connect: false,
        above_the_socket: vec![of_root(), of_root()],
        programs: vec![of_root(), of_root(), of_root()],
        configuration: Entry {
            mode: 0o700,
            ..of_root()
        },
        configuration_members: None,
        logins: Vec::new(),
        engine: LOWEST_ENGINE,
        compose: LOWEST_COMPOSE,
    }
}

/// An engine of the user 501, who is the accepted user of the daemon.
fn of_a_user() -> (World, StartFacts) {
    let mut world = World::new();
    let managed = world.settings.managed.as_mut().expect("on");
    managed.engine = EngineKind::User;
    managed.accepted_user = 501;
    let facts = StartFacts {
        running_as: 501,
        socket_owner: 501,
        configuration: Entry {
            owner: 501,
            mode: 0o700,
            ..of_root()
        },
        ..system()
    };
    (world, facts)
}

#[test]
fn k10_6_4_1_the_program_starts_on_a_system_engine_and_on_an_engine_of_a_user() {
    let world = World::new();
    assert_eq!(decided(&world.settings, &system()), Ok(EngineKind::System));
    let (world, facts) = of_a_user();
    assert_eq!(decided(&world.settings, &facts), Ok(EngineKind::User));
}

#[test]
fn k10_6_4_1_it_refuses_to_start_while_manage_applications_is_off() {
    let mut world = World::new();
    world.settings.managed = None;
    assert_eq!(decided(&world.settings, &system()), Err(StartRefusal::Off));
}

#[test]
fn k10_6_4_1_the_owner_of_the_real_socket_is_the_user_that_the_program_runs_as() {
    let world = World::new();
    // The program as the administrator with the engine of a user (SF-035).
    let facts = StartFacts {
        socket_owner: 501,
        ..system()
    };
    assert_eq!(
        decided(&world.settings, &facts),
        Err(StartRefusal::SocketOwner)
    );
    let not_a_socket = StartFacts {
        socket_is_a_socket: false,
        ..system()
    };
    assert_eq!(
        decided(&world.settings, &not_a_socket),
        Err(StartRefusal::NotASocket)
    );
    assert!(socket_owner_is_the_same(0, 0));
    assert!(!socket_owner_is_the_same(0, 501));
}

#[test]
fn k10_6_4_1_the_kind_of_the_engine_is_the_recorded_one() {
    let (mut world, facts) = of_a_user();
    world.settings.managed.as_mut().expect("on").engine = EngineKind::System;
    assert_eq!(
        decided(&world.settings, &facts),
        Err(StartRefusal::KindDifferent)
    );
    let mut world = World::new();
    world.settings.managed.as_mut().expect("on").engine = EngineKind::User;
    assert_eq!(
        decided(&world.settings, &system()),
        Err(StartRefusal::KindDifferent)
    );
}

#[test]
fn k10_6_4_1_on_an_engine_of_a_user_the_accepted_user_is_the_owner_of_the_engine() {
    let (mut world, facts) = of_a_user();
    world.settings.managed.as_mut().expect("on").accepted_user = 502;
    assert_eq!(
        decided(&world.settings, &facts),
        Err(StartRefusal::AcceptedUserDifferent)
    );
}

#[test]
fn k10_6_4_1_on_a_system_engine_the_accepted_user_cannot_connect_to_the_engine() {
    let mut world = World::new();
    let facts = StartFacts {
        accepted_user_can_connect: true,
        ..system()
    };
    assert_eq!(
        decided(&world.settings, &facts),
        Err(StartRefusal::AcceptedUserConnects)
    );
    world.settings.managed.as_mut().expect("on").accepted_user = 0;
    assert_eq!(
        decided(&world.settings, &system()),
        Err(StartRefusal::AcceptedUserConnects)
    );
}

#[test]
fn k10_6_4_1_a_path_that_a_user_who_is_not_permitted_can_write_refuses_the_start() {
    let world = World::new();
    let writable = [
        // The owner is neither the identifier 0, nor an administrator account, nor the user
        // that the program runs as.
        Entry {
            owner: 1000,
            ..of_root()
        },
        Entry {
            mode: 0o757,
            ..of_root()
        },
        Entry {
            mode: 0o775,
            group_members: vec![501, 1000],
            ..of_root()
        },
        Entry {
            listed_writers: vec![1000],
            ..of_root()
        },
        Entry {
            listed_for_all: true,
            ..of_root()
        },
    ];
    for entry in writable {
        let mut facts = system();
        facts.programs[1] = entry.clone();
        assert_eq!(
            decided(&world.settings, &facts),
            Err(StartRefusal::ProgramWritable)
        );
        let mut facts = system();
        facts.above_the_socket[0] = entry;
        assert_eq!(
            decided(&world.settings, &facts),
            Err(StartRefusal::SocketPathWritable)
        );
    }
}

#[test]
fn k10_6_4_1_a_path_that_only_permitted_users_can_write_does_not_refuse_the_start() {
    let world = World::new();
    let permitted = [
        // An administrator account owns it; a group whose members are each permitted.
        Entry {
            owner: 501,
            ..of_root()
        },
        Entry {
            mode: 0o775,
            group_members: vec![0, 501],
            ..of_root()
        },
        Entry {
            mode: 0o775,
            ..of_root()
        },
        Entry {
            listed_writers: vec![501],
            ..of_root()
        },
    ];
    for entry in permitted {
        let mut facts = system();
        facts.programs[1] = entry.clone();
        facts.above_the_socket[0] = entry;
        assert_eq!(decided(&world.settings, &facts), Ok(EngineKind::System));
    }
}

#[test]
fn k10_6_4_1_only_the_user_of_the_program_can_write_the_configuration_folder() {
    let (world, sound) = of_a_user();
    for configuration in [
        Entry {
            owner: 502,
            mode: 0o700,
            ..of_root()
        },
        Entry {
            owner: 501,
            mode: 0o770,
            group_members: vec![502],
            ..of_root()
        },
        Entry {
            owner: 501,
            mode: 0o702,
            ..of_root()
        },
        // Not even an administrator account.
        Entry {
            owner: 501,
            mode: 0o700,
            listed_writers: vec![0],
            ..of_root()
        },
    ] {
        let facts = StartFacts {
            configuration,
            ..sound.clone()
        };
        assert_eq!(
            decided(&world.settings, &facts),
            Err(StartRefusal::ConfigurationWritable)
        );
    }
}

#[test]
fn k10_6_4_1_config_json_has_the_member_auths_only_and_only_named_registries() {
    let mut world = World::new();
    let with = |members: &[&str], logins: &[&str]| StartFacts {
        configuration_members: Some(members.iter().map(|member| (*member).to_owned()).collect()),
        logins: logins.iter().map(|login| (*login).to_owned()).collect(),
        ..system()
    };
    assert_eq!(
        decided(&world.settings, &with(&["auths"], &[])),
        Ok(EngineKind::System)
    );
    for members in [
        &["auths", "credsStore"][..],
        &["credHelpers"],
        &["proxies", "auths"],
    ] {
        let refused = decided(&world.settings, &with(members, &[]));
        assert_eq!(refused, Err(StartRefusal::ConfigurationMembers));
    }
    let login = with(&["auths"], &["ghcr.io"]);
    assert_eq!(
        decided(&world.settings, &login),
        Err(StartRefusal::LoginNotNamed)
    );
    world.settings.registries = vec!["ghcr.io".to_owned()];
    assert_eq!(decided(&world.settings, &login), Ok(EngineKind::System));
}

#[test]
fn k10_6_4_1_an_engine_or_a_compose_program_older_than_the_lowest_version_refuses_the_start() {
    let world = World::new();
    let old_engine = StartFacts {
        engine: Version(27, 5, 1),
        ..system()
    };
    assert_eq!(
        decided(&world.settings, &old_engine),
        Err(StartRefusal::EngineTooOld)
    );
    let old_compose = StartFacts {
        compose: Version(2, 23, 0),
        ..system()
    };
    assert_eq!(
        decided(&world.settings, &old_compose),
        Err(StartRefusal::ComposeTooOld)
    );
    let newer = StartFacts {
        engine: Version(29, 9, 3),
        compose: Version(5, 7, 0),
        ..system()
    };
    assert_eq!(decided(&world.settings, &newer), Ok(EngineKind::System));
}
