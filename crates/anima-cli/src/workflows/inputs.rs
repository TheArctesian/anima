//! The named inputs of a workflow started by hand, from `--input name=value` (PRJ-035). A value
//! can be anything the operator types, a secret included, so a failure names the input by its
//! place or its name and never shows a value (LOG-007).

use crate::failure::Failure;

/// The most inputs GitHub takes.
const MAX_INPUTS: usize = 25;
/// The longest name the hub takes.
const MAX_NAME: usize = 100;
/// The longest value the hub takes, in characters.
const MAX_VALUE: usize = 1_024;

/// One named input.
#[derive(Clone, PartialEq, Eq)]
pub struct Input {
    pub name: String,
    pub value: String,
}

/// Never the value, so a stray debug print cannot show it.
impl std::fmt::Debug for Input {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Input")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

fn usage(message: String) -> Failure {
    Failure::Usage {
        message,
        hint: None,
    }
}

/// Whether `name` is 1 to 100 letters, digits, `-` and `_`.
fn is_name(name: &str) -> bool {
    (1..=MAX_NAME).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// The inputs in the order given: each `name=value` split at its first `=`.
///
/// # Errors
/// [`Failure::Usage`] for more than 25, one with no `=` or a name that is not 1 to 100 of
/// letters, digits, `-` and `_` (named by its place), a value over 1,024 characters or a name
/// given twice (named by its name).
pub fn parse(given: &[String]) -> Result<Vec<Input>, Failure> {
    if given.len() > MAX_INPUTS {
        return Err(usage(format!(
            "A workflow takes at most {MAX_INPUTS} inputs; {} were given.",
            given.len()
        )));
    }
    let mut inputs: Vec<Input> = Vec::with_capacity(given.len());
    for (place, text) in given.iter().enumerate() {
        let place = place + 1;
        let Some((name, value)) = text.split_once('=').filter(|(name, _)| is_name(name)) else {
            return Err(usage(format!(
                "--input number {place} is not name=value with a name of 1 to {MAX_NAME} letters, digits, - and _."
            )));
        };
        if value.chars().count() > MAX_VALUE {
            return Err(usage(format!(
                "The value of input {name} is longer than {MAX_VALUE} characters."
            )));
        }
        if inputs.iter().any(|input| input.name == name) {
            return Err(usage(format!("The input {name} is given twice.")));
        }
        inputs.push(Input {
            name: name.to_owned(),
            value: value.to_owned(),
        });
    }
    Ok(inputs)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const MARK: &str = "S3CR3T";

    #[test]
    fn prj_035_inputs_split_at_the_first_equals_sign_in_order() {
        let given = vec![
            "tag=v1.2".to_owned(),
            "query=a=b".to_owned(),
            "empty=".to_owned(),
        ];
        let inputs = parse(&given).expect("inputs");
        let pairs: Vec<(&str, &str)> = inputs
            .iter()
            .map(|input| (input.name.as_str(), input.value.as_str()))
            .collect();
        assert_eq!(pairs, [("tag", "v1.2"), ("query", "a=b"), ("empty", "")]);
        assert!(!format!("{inputs:?}").contains("v1.2"));
    }

    #[test]
    fn prj_035_too_many_inputs_or_a_name_twice_is_refused() {
        let many: Vec<String> = (0..26).map(|n| format!("n{n}=x")).collect();
        assert_eq!(parse(&many).expect_err("26").exit_code(), 2);
        let twice = vec![format!("a={MARK}"), format!("a={MARK}2")];
        let failure = parse(&twice).expect_err("twice");
        assert!(failure.to_string().contains("input a "), "{failure}");
        assert!(!failure.to_string().contains(MARK), "{failure}");
    }

    proptest! {
        #[test]
        fn log_007_a_good_input_comes_back_exactly(
            name in "[A-Za-z0-9_-]{1,100}",
            value in "\\PC{0,200}",
        ) {
            let inputs = parse(&[format!("{name}={value}")]).expect("a good input");
            prop_assert_eq!(&inputs[0].name, &name);
            prop_assert_eq!(&inputs[0].value, &value);
        }

        #[test]
        fn log_007_no_refusal_shows_a_value(
            before in "\\PC{0,30}",
            after in "\\PC{0,30}",
            long in 1_025usize..1_100,
        ) {
            let secret = format!("{MARK}{after}");
            let refused = [
                format!("{before}{secret}"),
                format!("bad name{before}={secret}"),
                format!("={secret}"),
                format!("ok={secret}{}", "v".repeat(long)),
            ];
            for text in refused {
                if let Err(failure) = parse(&[text]) {
                    prop_assert!(!failure.to_string().contains(MARK), "{}", failure);
                }
            }
        }
    }
}
