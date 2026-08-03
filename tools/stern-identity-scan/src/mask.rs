#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConfiguredRepository {
    pub(crate) owner: String,
    pub(crate) slug: String,
    pub(crate) repository: String,
}

impl ConfiguredRepository {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self, String> {
        let value = value.unwrap_or_default();
        let mut pieces = value.split('/');
        let owner = pieces.next().unwrap_or_default();
        let slug = pieces.next().unwrap_or_default();
        if pieces.next().is_some()
            || owner.is_empty()
            || slug != "stern"
            || !owner.chars().all(is_repository_character)
            || !slug.chars().all(is_repository_character)
        {
            return Err("configured repository must name the canonical repository".to_owned());
        }
        Ok(Self {
            owner: owner.to_owned(),
            slug: slug.to_owned(),
            repository: value.to_owned(),
        })
    }
}

fn is_repository_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
}

pub(crate) fn mask_canonical_repository_urls(
    value: &str,
    configured: &ConfiguredRepository,
) -> String {
    let https = format!(
        "https://github.com/{}/{}",
        configured.owner, configured.slug
    );
    let git = format!(
        "git@github.com:{}/{}.git",
        configured.owner, configured.slug
    );
    let ssh = format!(
        "ssh://git@github.com/{}/{}.git",
        configured.owner, configured.slug
    );
    let mut masked = mask_pattern(value, &https, configured, https_boundary);
    masked = mask_pattern(&masked, &git, configured, remote_boundary);
    mask_pattern(&masked, &ssh, configured, remote_boundary)
}

pub(crate) fn mask_metadata_value(
    key: &str,
    value: &str,
    configured: &ConfiguredRepository,
) -> String {
    if key == "repository" && value == configured.repository {
        return mask_owner(value, &configured.owner);
    }
    let exact_remotes = [
        format!("https://github.com/{}.git", configured.repository),
        format!("git@github.com:{}.git", configured.repository),
        format!("ssh://git@github.com/{}.git", configured.repository),
    ];
    if key == "origin" && exact_remotes.iter().any(|remote| remote == value) {
        return mask_owner(value, &configured.owner);
    }
    value.to_owned()
}

fn mask_pattern(
    value: &str,
    pattern: &str,
    configured: &ConfiguredRepository,
    is_boundary: fn(Option<char>) -> bool,
) -> String {
    let mut result = String::with_capacity(value.len());
    let mut cursor = 0;
    while let Some(relative_start) = value[cursor..].find(pattern) {
        let start = cursor + relative_start;
        let end = start + pattern.len();
        result.push_str(&value[cursor..start]);
        if is_boundary(value[end..].chars().next()) {
            result.push_str(&mask_owner(pattern, &configured.owner));
        } else {
            result.push_str(pattern);
        }
        cursor = end;
    }
    result.push_str(&value[cursor..]);
    result
}

fn mask_owner(value: &str, owner: &str) -> String {
    value.replace(owner, &"x".repeat(owner.len()))
}

fn https_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| {
        is_ecmascript_whitespace(character)
            || matches!(
                character,
                '/' | '?' | '#' | '"' | '\'' | ')' | ']' | '>' | ',' | '.' | ';' | ':'
            )
    })
}

fn remote_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| {
        is_ecmascript_whitespace(character)
            || matches!(
                character,
                '"' | '\'' | ')' | ']' | '>' | ',' | '.' | ';' | ':'
            )
    })
}

fn is_ecmascript_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marker::first_forbidden_match;

    fn configured() -> ConfiguredRepository {
        let owner = [107_u8, 105, 110, 101, 116, 105, 107, 45, 103, 103]
            .into_iter()
            .map(char::from)
            .collect::<String>();
        ConfiguredRepository::parse(Some(&format!("{owner}/stern"))).unwrap()
    }

    #[test]
    fn validates_only_owner_and_canonical_slug() {
        assert!(ConfiguredRepository::parse(Some("owner/stern")).is_ok());
        for invalid in [
            None,
            Some(""),
            Some("owner"),
            Some("owner/other"),
            Some("a/b/stern"),
            Some("owner!/stern"),
        ] {
            assert_eq!(
                ConfiguredRepository::parse(invalid),
                Err("configured repository must name the canonical repository".to_owned())
            );
        }
    }

    #[test]
    fn masks_canonical_https_boundaries() {
        let configured = configured();
        let canonical = format!("https://github.com/{}", configured.repository);
        for suffix in ["", "/issues/1", "?tab=readme", "#readme", ".git", ")"] {
            let masked =
                mask_canonical_repository_urls(&format!("{canonical}{suffix}"), &configured);
            assert!(
                first_forbidden_match(&masked).is_none(),
                "suffix {suffix:?}"
            );
            assert_eq!(masked.len(), canonical.len() + suffix.len());
        }
    }

    #[test]
    fn masks_canonical_ssh_remotes_only_at_boundaries() {
        let configured = configured();
        for canonical in [
            format!("git@github.com:{}.git", configured.repository),
            format!("ssh://git@github.com/{}.git", configured.repository),
        ] {
            assert!(
                first_forbidden_match(&mask_canonical_repository_urls(&canonical, &configured))
                    .is_none()
            );
            assert!(
                first_forbidden_match(&mask_canonical_repository_urls(
                    &format!("{canonical}x"),
                    &configured
                ))
                .is_some()
            );
        }
    }

    #[test]
    fn masks_all_ecmascript_whitespace_boundaries_for_every_url_form() {
        let configured = configured();
        let forms = [
            format!("https://github.com/{}", configured.repository),
            format!("git@github.com:{}.git", configured.repository),
            format!("ssh://git@github.com/{}.git", configured.repository),
        ];
        let whitespace = [
            '\u{0009}', '\u{000A}', '\u{000B}', '\u{000C}', '\u{000D}', '\u{0020}', '\u{00A0}',
            '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
            '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{2028}', '\u{2029}',
            '\u{202F}', '\u{205F}', '\u{3000}', '\u{FEFF}',
        ];
        for form in forms {
            for boundary in whitespace {
                let value = format!("{form}{boundary}tail");
                assert!(
                    first_forbidden_match(&mask_canonical_repository_urls(&value, &configured))
                        .is_none(),
                    "form {form:?}, boundary U+{:04X}",
                    u32::from(boundary)
                );
            }
        }
    }

    #[test]
    fn keeps_non_ecmascript_whitespace_as_a_non_boundary() {
        let configured = configured();
        for form in [
            format!("https://github.com/{}", configured.repository),
            format!("git@github.com:{}.git", configured.repository),
            format!("ssh://git@github.com/{}.git", configured.repository),
        ] {
            assert!(
                first_forbidden_match(&mask_canonical_repository_urls(
                    &format!("{form}\u{200B}"),
                    &configured,
                ))
                .is_some()
            );
        }
    }

    #[test]
    fn leaves_noncanonical_occurrences_visible() {
        let configured = configured();
        for value in [
            format!("https://example.com/{}", configured.repository),
            format!("https://github.com/{}x", configured.repository),
            configured.repository.clone(),
        ] {
            assert!(
                first_forbidden_match(&mask_canonical_repository_urls(&value, &configured))
                    .is_some()
            );
        }
    }

    #[test]
    fn masks_exact_repository_and_origin_metadata() {
        let configured = configured();
        assert!(
            first_forbidden_match(&mask_metadata_value(
                "repository",
                &configured.repository,
                &configured
            ))
            .is_none()
        );
        for origin in [
            format!("https://github.com/{}.git", configured.repository),
            format!("git@github.com:{}.git", configured.repository),
            format!("ssh://git@github.com/{}.git", configured.repository),
        ] {
            assert!(
                first_forbidden_match(&mask_metadata_value("origin", &origin, &configured))
                    .is_none()
            );
        }
        assert!(
            first_forbidden_match(&mask_metadata_value(
                "owner",
                &configured.owner,
                &configured
            ))
            .is_some()
        );
    }
}
