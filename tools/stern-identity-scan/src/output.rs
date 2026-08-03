use std::io::Write;

use crate::marker::{ForbiddenMarker, sha256};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MatchRecord {
    Path {
        kind: String,
        surface: PathSurface,
        marker: ForbiddenMarker,
    },
    Metadata {
        key_sha256: String,
        marker: ForbiddenMarker,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PathSurface {
    Path(String),
    Sha256(String),
}

impl MatchRecord {
    pub(crate) fn path(kind: &str, path: &str, marker: ForbiddenMarker) -> Self {
        let normalized = path.replace('\\', "/");
        let surface = if crate::marker::first_forbidden_match(path).is_some() {
            PathSurface::Sha256(sha256(&normalized))
        } else {
            PathSurface::Path(normalized)
        };
        Self::Path {
            kind: kind.to_owned(),
            surface,
            marker,
        }
    }

    pub(crate) fn metadata(key: &str, marker: ForbiddenMarker) -> Self {
        Self::Metadata {
            key_sha256: sha256(key),
            marker,
        }
    }

    fn to_json(&self) -> String {
        match self {
            Self::Path {
                kind,
                surface,
                marker,
            } => {
                let surface = match surface {
                    PathSurface::Path(path) => format!("\"path\":{}", json_string(path)),
                    PathSurface::Sha256(hash) => {
                        format!("\"pathSha256\":{}", json_string(hash))
                    }
                };
                format!(
                    "{{\"kind\":{},{surface},\"normalizedLength\":{},\"markerHash\":{}}}",
                    json_string(kind),
                    marker.normalized_length,
                    json_string(marker.sha256)
                )
            }
            Self::Metadata { key_sha256, marker } => format!(
                "{{\"kind\":\"metadata\",\"keySha256\":{},\"normalizedLength\":{},\"markerHash\":{}}}",
                json_string(key_sha256),
                marker.normalized_length,
                json_string(marker.sha256)
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProcessOutput {
    pub(crate) exit_code: i32,
    pub(crate) stdout: Option<String>,
    pub(crate) stderr: Option<String>,
}

impl ProcessOutput {
    pub(crate) fn self_test_success() -> Self {
        Self::stdout(
            0,
            "{\"status\":\"pass\",\"suite\":\"identity-scan\",\"tests\":11}".to_owned(),
        )
    }

    pub(crate) fn scan_success(scope: &str, text_files: usize) -> Self {
        Self::stdout(
            0,
            format!(
                "{{\"status\":\"pass\",\"scope\":{},\"textFiles\":{text_files}}}",
                json_string(scope)
            ),
        )
    }

    pub(crate) fn violation(record: &MatchRecord) -> Self {
        Self::stderr(
            1,
            format!("{{\"status\":\"fail\",\"match\":{}}}", record.to_json()),
        )
    }

    pub(crate) fn error(message: &str) -> Self {
        Self::stderr(
            2,
            format!(
                "{{\"status\":\"error\",\"message\":{}}}",
                json_string(message)
            ),
        )
    }

    fn stdout(exit_code: i32, stdout: String) -> Self {
        Self {
            exit_code,
            stdout: Some(stdout),
            stderr: None,
        }
    }

    fn stderr(exit_code: i32, stderr: String) -> Self {
        Self {
            exit_code,
            stdout: None,
            stderr: Some(stderr),
        }
    }

    pub(crate) fn write_to(&self, stdout: &mut impl Write, stderr: &mut impl Write) {
        if let Some(line) = &self.stdout {
            let _ = writeln!(stdout, "{line}");
        }
        if let Some(line) = &self.stderr {
            let _ = writeln!(stderr, "{line}");
        }
    }
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("strings always serialize to JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marker::first_forbidden_match;

    fn marker_text() -> String {
        [107_u8, 105, 110, 101, 116, 105, 107]
            .into_iter()
            .map(char::from)
            .collect()
    }

    #[test]
    fn safe_paths_are_displayed_with_forward_slashes() {
        let marker = first_forbidden_match(&marker_text()).unwrap();
        let record = MatchRecord::path("content", "safe\\file.txt", marker);
        assert!(record.to_json().contains("\"path\":\"safe/file.txt\""));
    }

    #[test]
    fn unsafe_paths_are_only_hashed() {
        let forbidden = marker_text();
        let marker = first_forbidden_match(&forbidden).unwrap();
        let path = format!("private\\{forbidden}.txt");
        let record = MatchRecord::path("path", &path, marker);
        let json = record.to_json();
        assert!(json.contains("pathSha256"));
        assert!(!json.contains(&forbidden));
        assert_eq!(json.matches(marker.sha256).count(), 1);
    }

    #[test]
    fn error_strings_are_json_escaped() {
        assert_eq!(
            ProcessOutput::error("bad \"input\"").stderr.as_deref(),
            Some("{\"status\":\"error\",\"message\":\"bad \\\"input\\\"\"}")
        );
    }
}
