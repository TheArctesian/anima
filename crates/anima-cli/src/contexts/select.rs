//! Which hub a command talks to (CMD-013, CMD-015). The first that is given wins: `--hub`,
//! `--context`, `ANIMA_HUB`, the current context, then the default hub.

use super::config::Config;
use crate::failure::{Failure, Hint};
use crate::hub::Origin;

/// What the operator gave to choose a hub.
#[derive(Clone, Copy, Debug, Default)]
pub struct Selection<'a> {
    /// `--hub`.
    pub hub: Option<&'a str>,
    /// `--context`.
    pub context: Option<&'a str>,
    /// `ANIMA_HUB`.
    pub hub_variable: Option<&'a str>,
}

/// The hub `selection` and `config` choose.
///
/// # Errors
/// [`Failure::Usage`] for a hub address that is not an `https` origin; [`Failure::NotFound`]
/// for a context that does not exist.
pub fn select(selection: Selection<'_>, config: &Config) -> Result<Origin, Failure> {
    if let Some(text) = selection.hub {
        return parse("--hub", text);
    }
    if let Some(name) = selection.context {
        return named(config, name);
    }
    if let Some(text) = selection.hub_variable {
        return parse("ANIMA_HUB", text);
    }
    match &config.current {
        Some(name) => named(config, name),
        None => Ok(Origin::default_hub()),
    }
}

fn parse(source: &str, text: &str) -> Result<Origin, Failure> {
    Origin::parse(text).map_err(|error| Failure::Usage {
        message: format!("{source}: {error}."),
        hint: None,
    })
}

fn named(config: &Config, name: &str) -> Result<Origin, Failure> {
    config.hub_of(name)?.ok_or_else(|| Failure::NotFound {
        message: format!("There is no context named {name}."),
        hint: Some(Hint::run("anima context list")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::config::Context;

    fn config() -> Config {
        let mut config = Config {
            current: Some("home".into()),
            ..Config::default()
        };
        for (name, hub) in [
            ("home", "https://home.example"),
            ("lab", "https://lab.example"),
        ] {
            config
                .contexts
                .insert(name.into(), Context { hub: hub.into() });
        }
        config
    }

    fn chosen(selection: Selection<'_>, config: &Config) -> String {
        select(selection, config).expect("a hub").as_text()
    }

    #[test]
    fn cmd_013_the_flag_then_the_context_then_the_variable_then_the_current_context() {
        let all = Selection {
            hub: Some("https://flag.example"),
            context: Some("lab"),
            hub_variable: Some("https://variable.example"),
        };
        assert_eq!(chosen(all, &config()), "https://flag.example");
        let no_flag = Selection { hub: None, ..all };
        assert_eq!(chosen(no_flag, &config()), "https://lab.example");
        let variable = Selection {
            context: None,
            ..no_flag
        };
        assert_eq!(chosen(variable, &config()), "https://variable.example");
        assert_eq!(
            chosen(Selection::default(), &config()),
            "https://home.example"
        );
        assert_eq!(
            chosen(Selection::default(), &Config::default()),
            "https://hub.anima.ing"
        );
    }

    #[test]
    fn cmd_015_an_unknown_context_is_not_found_and_a_plain_http_hub_is_wrong_usage() {
        let unknown = Selection {
            context: Some("nope"),
            ..Selection::default()
        };
        assert_eq!(select(unknown, &config()).expect_err("x").exit_code(), 3);
        let http = Selection {
            hub: Some("http://hub.example"),
            ..Selection::default()
        };
        assert_eq!(select(http, &config()).expect_err("x").exit_code(), 2);
    }
}
