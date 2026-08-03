//! Deterministic Rust constant naming and collision checks.

use std::collections::BTreeMap;

use crate::{Error, ErrorKind, Result};

/// A source name paired with its generated constant identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstantName {
    /// Original catalog name or alias.
    pub source: String,
    /// Uppercase snake-case Rust identifier.
    pub rust: String,
}

/// Converts a catalog name to a valid uppercase snake-case Rust identifier.
#[must_use]
pub fn constant_name(source: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in source.chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('_');
            }
            result.push(character.to_ascii_uppercase());
            separator = false;
        } else {
            separator = true;
        }
    }
    if result.is_empty() {
        result.push_str("ICON");
    }
    if result.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        result.insert_str(0, "ICON_");
    }
    // Uppercase identifiers cannot equal lowercase Rust keywords, but this
    // suffix makes the rule explicit and robust if naming policy changes.
    if is_keyword(&result.to_ascii_lowercase()) {
        result.push_str("_ICON");
    }
    result
}

/// Assigns names and fails if canonical names or aliases collide.
///
/// # Errors
///
/// Returns [`ErrorKind::NameCollision`](crate::ErrorKind::NameCollision) when
/// distinct source names normalize to the same Rust identifier.
pub fn assign_constant_names<'a>(
    sources: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<ConstantName>> {
    let mut assigned = BTreeMap::<String, String>::new();
    for source in sources {
        let rust = constant_name(source);
        if let Some(previous) = assigned.insert(rust.clone(), source.to_owned()) {
            return Err(Error::new(
                ErrorKind::NameCollision,
                rust,
                format!("source names `{previous}` and `{source}` normalize identically"),
            ));
        }
    }
    Ok(assigned
        .into_iter()
        .map(|(rust, source)| ConstantName { source, rust })
        .collect())
}

fn is_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
    )
}
