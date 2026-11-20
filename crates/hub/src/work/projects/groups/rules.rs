//! What a group name and a list of projects must be (PRO-059, PRO-060). Pure.

/// The longest group name, in characters.
pub const NAME_CHARS: usize = 60;
/// The most projects that one change names.
pub const MOST_PROJECTS: usize = 200;

/// The name as it is stored: the text without the spaces at its two ends. `None` when it is
/// empty, longer than [`NAME_CHARS`] or holds a control character.
#[must_use]
pub fn name(text: &str) -> Option<String> {
    let name = text.trim();
    let chars = name.chars().count();
    ((1..=NAME_CHARS).contains(&chars) && !name.chars().any(char::is_control))
        .then(|| name.to_owned())
}

/// The ids of `given` without the second of two that are the same, in their order. `None` when
/// there are more than [`MOST_PROJECTS`] of them.
#[must_use]
pub fn distinct<T: PartialEq + Copy>(given: &[T]) -> Option<Vec<T>> {
    let mut kept: Vec<T> = Vec::new();
    for each in given {
        if !kept.contains(each) {
            kept.push(*each);
        }
    }
    (kept.len() <= MOST_PROJECTS).then_some(kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pro_059_a_name_is_1_to_60_characters_with_no_control_character() {
        assert_eq!(name("  Work  ").as_deref(), Some("Work"));
        assert_eq!(name("   "), None);
        assert_eq!(name(&"é".repeat(60)).map(|n| n.chars().count()), Some(60));
        assert_eq!(name(&"a".repeat(61)), None);
        assert_eq!(name("a\nb"), None);
    }

    #[test]
    fn pro_060_a_list_keeps_one_of_each_and_at_most_200() {
        assert_eq!(distinct(&[1, 2, 1, 3]), Some(vec![1, 2, 3]));
        let many: Vec<usize> = (0..=MOST_PROJECTS).collect();
        assert_eq!(distinct(&many), None);
        assert_eq!(
            distinct(&many[1..]).map(|kept| kept.len()),
            Some(MOST_PROJECTS)
        );
    }
}
