use std::fmt::Write as _;

use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ForbiddenMarker {
    pub(crate) normalized_length: usize,
    pub(crate) sha256: &'static str,
}

const FORBIDDEN_MARKERS: [ForbiddenMarker; 2] = [
    ForbiddenMarker {
        normalized_length: 7,
        sha256: "d1f8b80df0f04f9fb9d2301f8811562cc16010891492393f88ec8c3a4f2890a5",
    },
    ForbiddenMarker {
        normalized_length: 9,
        sha256: "7c0ddb175d6f4cf0264ff2a86c84492826e61450889b5c78d108ffa6ecd1c563",
    },
];

pub(crate) fn sha256(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a string cannot fail");
    }
    encoded
}

pub(crate) fn first_forbidden_match(value: &str) -> Option<ForbiddenMarker> {
    let folded = ascii_case_fold(value);
    for run in folded.split(|byte: char| !byte.is_ascii_lowercase() && !byte.is_ascii_digit()) {
        for marker in FORBIDDEN_MARKERS {
            if run.len() < marker.normalized_length {
                continue;
            }
            for offset in 0..=run.len() - marker.normalized_length {
                let candidate = &run[offset..offset + marker.normalized_length];
                if sha256(candidate) == marker.sha256 {
                    return Some(marker);
                }
            }
        }
    }
    None
}

fn ascii_case_fold(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_uppercase() {
                byte + (b'a' - b'A')
            } else {
                byte
            }
        })
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker_base() -> String {
        [107_u8, 105, 110, 101, 116, 105, 107]
            .into_iter()
            .map(char::from)
            .collect()
    }

    fn marker_suffix() -> String {
        [117_u8, 105].into_iter().map(char::from).collect()
    }

    #[test]
    fn detects_both_marker_lengths_without_storing_marker_text() {
        let base = marker_base();
        let long = format!("{base}{}", marker_suffix());
        assert_eq!(first_forbidden_match(&base).unwrap().normalized_length, 7);
        assert_eq!(first_forbidden_match(&long).unwrap().normalized_length, 7);
        assert_eq!(sha256(&long), FORBIDDEN_MARKERS[1].sha256);
        assert_eq!(FORBIDDEN_MARKERS[1].normalized_length, 9);
    }

    #[test]
    fn folds_ascii_case_and_finds_contiguous_subsequences() {
        let base = marker_base();
        assert!(first_forbidden_match(&base.to_ascii_uppercase()).is_some());
        assert!(first_forbidden_match(&format!("prefix{base}suffix")).is_some());
    }

    #[test]
    fn punctuation_separates_ascii_runs() {
        let base = marker_base();
        assert!(first_forbidden_match(&format!("{base}-{}", marker_suffix())).is_some());
        let split = format!("{}-{}", &base[..3], &base[3..]);
        assert!(first_forbidden_match(&split).is_none());
    }

    #[test]
    fn clean_unicode_is_not_case_folded_into_a_marker() {
        assert!(first_forbidden_match("Stern — UTF-8 — KİNETİK").is_none());
    }

    #[test]
    fn hashes_match_known_non_disclosing_values() {
        assert_eq!(
            sha256("owner"),
            "4c1029697ee358715d3a14a2add817c4b01651440de808371f78165ac90dc581"
        );
    }
}
