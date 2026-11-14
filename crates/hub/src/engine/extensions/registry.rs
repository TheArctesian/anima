//! The fifteen extensions of Section 1.1 of the extensions plan, as data (K-E7, EXT-006,
//! EXT-010). Only Projects is built; Integrations is part of the engine and always on.

use super::model::{Entry, Key, Navigation, NavigationGroup, Sensitivity};

const MAP: Key = Key::new("map");
pub const PROJECTS: Key = Key::new("projects");
const WRITING: Key = Key::new("writing");
const SECRETS: Key = Key::new("secrets");
const INTEGRATIONS: Key = Key::new("integrations");
const PEOPLE: Key = Key::new("people");
const MONEY: Key = Key::new("money");
const HISTORY: Key = Key::new("history");
const HEALTH: Key = Key::new("health");
const DOCUMENTS: Key = Key::new("documents");
const REMINDERS: Key = Key::new("reminders");
const ASSISTANT_SESSIONS: Key = Key::new("assistant_sessions");
const WARDROBE: Key = Key::new("wardrobe");
const LIBRARY: Key = Key::new("library");
const MODEL_GATEWAY: Key = Key::new("model_gateway");

static REGISTRY: [Entry; 15] = [
    Entry {
        key: MAP,
        name: "Map",
        data_class: Sensitivity::B,
        needs: &[],
        works_with: &[HEALTH, HISTORY, PEOPLE],
        navigation: Some(Navigation {
            group: NavigationGroup::Life,
            label: "Map",
            route: "/map",
        }),
        migrations: Some((2100, 2199)),
        built: false,
        fixed: false,
        setting: "extensions.map",
    },
    Entry {
        key: PROJECTS,
        name: "Projects",
        data_class: Sensitivity::C,
        needs: &[],
        works_with: &[SECRETS, MODEL_GATEWAY, ASSISTANT_SESSIONS, WRITING],
        navigation: Some(Navigation {
            group: NavigationGroup::Infrastructure,
            label: "Projects",
            route: "/projects",
        }),
        migrations: Some((2200, 2299)),
        built: true,
        fixed: false,
        setting: "extensions.projects",
    },
    Entry {
        key: WRITING,
        name: "Writing",
        data_class: Sensitivity::C,
        needs: &[],
        works_with: &[PROJECTS],
        navigation: Some(Navigation {
            group: NavigationGroup::Work,
            label: "Writing",
            route: "/writing",
        }),
        migrations: Some((2300, 2399)),
        built: false,
        fixed: false,
        setting: "extensions.writing",
    },
    Entry {
        key: SECRETS,
        name: "Secrets",
        data_class: Sensitivity::B,
        needs: &[],
        works_with: &[PROJECTS],
        navigation: Some(Navigation {
            group: NavigationGroup::Vault,
            label: "Secrets",
            route: "/secrets",
        }),
        migrations: Some((2400, 2499)),
        built: false,
        fixed: false,
        setting: "extensions.secrets",
    },
    Entry {
        key: INTEGRATIONS,
        name: "Integrations",
        data_class: Sensitivity::C,
        needs: &[],
        works_with: &[],
        navigation: None,
        migrations: Some((2000, 2019)),
        built: true,
        fixed: true,
        setting: "extensions.integrations",
    },
    Entry {
        key: PEOPLE,
        name: "People",
        data_class: Sensitivity::C,
        needs: &[],
        works_with: &[MAP, LIBRARY],
        navigation: Some(Navigation {
            group: NavigationGroup::Life,
            label: "People",
            route: "/people",
        }),
        migrations: Some((2500, 2599)),
        built: false,
        fixed: false,
        setting: "extensions.people",
    },
    Entry {
        key: MONEY,
        name: "Money",
        data_class: Sensitivity::B,
        needs: &[],
        works_with: &[WARDROBE],
        navigation: Some(Navigation {
            group: NavigationGroup::Life,
            label: "Money",
            route: "/money",
        }),
        migrations: Some((2600, 2699)),
        built: false,
        fixed: false,
        setting: "extensions.money",
    },
    Entry {
        key: HISTORY,
        name: "History",
        data_class: Sensitivity::B,
        needs: &[],
        works_with: &[MAP],
        navigation: Some(Navigation {
            group: NavigationGroup::Life,
            label: "History",
            route: "/history",
        }),
        migrations: None,
        built: false,
        fixed: false,
        setting: "extensions.history",
    },
    Entry {
        key: HEALTH,
        name: "Health",
        data_class: Sensitivity::B,
        needs: &[],
        works_with: &[MAP],
        navigation: Some(Navigation {
            group: NavigationGroup::Life,
            label: "Health",
            route: "/health",
        }),
        migrations: Some((2800, 2899)),
        built: false,
        fixed: false,
        setting: "extensions.health",
    },
    Entry {
        key: DOCUMENTS,
        name: "Documents",
        data_class: Sensitivity::A,
        needs: &[],
        works_with: &[],
        navigation: Some(Navigation {
            group: NavigationGroup::Vault,
            label: "Documents",
            route: "/documents",
        }),
        migrations: Some((2900, 2999)),
        built: false,
        fixed: false,
        setting: "extensions.documents",
    },
    Entry {
        key: REMINDERS,
        name: "Reminders",
        data_class: Sensitivity::C,
        needs: &[],
        works_with: &[],
        navigation: Some(Navigation {
            group: NavigationGroup::Bottom,
            label: "Reminders",
            route: "/reminders",
        }),
        migrations: None,
        built: false,
        fixed: false,
        setting: "extensions.reminders",
    },
    Entry {
        key: ASSISTANT_SESSIONS,
        name: "AI coding sessions",
        data_class: Sensitivity::B,
        needs: &[],
        works_with: &[PROJECTS],
        navigation: None,
        migrations: Some((3100, 3199)),
        built: false,
        fixed: false,
        setting: "extensions.assistant_sessions",
    },
    Entry {
        key: WARDROBE,
        name: "Wardrobe",
        data_class: Sensitivity::C,
        needs: &[],
        works_with: &[MONEY],
        navigation: Some(Navigation {
            group: NavigationGroup::Belongings,
            label: "Wardrobe",
            route: "/wardrobe",
        }),
        migrations: Some((3200, 3299)),
        built: false,
        fixed: false,
        setting: "extensions.wardrobe",
    },
    Entry {
        key: LIBRARY,
        name: "Library",
        data_class: Sensitivity::C,
        needs: &[],
        works_with: &[PEOPLE],
        navigation: Some(Navigation {
            group: NavigationGroup::Belongings,
            label: "Library",
            route: "/library",
        }),
        migrations: Some((3300, 3399)),
        built: false,
        fixed: false,
        setting: "extensions.library",
    },
    Entry {
        key: MODEL_GATEWAY,
        name: "Model gateway",
        data_class: Sensitivity::C,
        needs: &[SECRETS],
        works_with: &[PROJECTS],
        navigation: Some(Navigation {
            group: NavigationGroup::Infrastructure,
            label: "Model gateway",
            route: "/model-gateway",
        }),
        migrations: Some((3500, 3599)),
        built: false,
        fixed: false,
        setting: "extensions.model_gateway",
    },
];

/// Every extension, in the order of the plan.
#[must_use]
pub fn registry() -> &'static [Entry] {
    &REGISTRY
}
