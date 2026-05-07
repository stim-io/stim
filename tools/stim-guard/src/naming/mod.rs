mod rust;
mod ts;

use crate::{
    config::MAX_NAME_WORDS,
    model::{issue, Issue, Severity},
};

pub(crate) use rust::check_rust_names;
pub(crate) use ts::check_ts_names;

pub(crate) fn check_name(issues: &mut Vec<Issue>, path: &str, line: usize, kind: &str, name: &str) {
    let word_count = count_name_words(name);
    if word_count <= MAX_NAME_WORDS {
        return;
    }

    issues.push(issue(
        Severity::Deny,
        "name-too-many-words",
        path,
        Some(line),
        format!("{kind} `{name}` has {word_count} words; max is {MAX_NAME_WORDS}"),
    ));
}

pub(crate) fn count_name_words(name: &str) -> usize {
    split_name_words(name).len()
}

fn split_name_words(name: &str) -> Vec<String> {
    let mut words = Vec::new();
    let normalized = name
        .strip_prefix("r#")
        .unwrap_or(name)
        .trim_matches('_')
        .trim_matches('$');

    for part in normalized
        .split(['_', '-', '$'])
        .filter(|part| !part.is_empty())
    {
        split_camel_part(part, &mut words);
    }

    words
}

fn split_camel_part(part: &str, words: &mut Vec<String>) {
    let chars: Vec<char> = part.chars().collect();
    let mut current = String::new();

    for (index, ch) in chars.iter().enumerate() {
        if should_start_word(&chars, index) && !current.is_empty() {
            words.push(current);
            current = String::new();
        }
        current.push(*ch);
    }

    if !current.is_empty() {
        words.push(current);
    }
}

fn should_start_word(chars: &[char], index: usize) -> bool {
    if index == 0 || !chars[index].is_uppercase() {
        return false;
    }

    let prev = chars[index - 1];
    let next = chars.get(index + 1).copied();

    prev.is_lowercase()
        || prev.is_ascii_digit()
        || (prev.is_uppercase() && next.is_some_and(char::is_lowercase))
}
