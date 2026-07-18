use std::fs;
use std::path::Path;

use crate::inventory::FileEntry;
use crate::marker::first_forbidden_match;
use crate::mask::{ConfiguredRepository, mask_canonical_repository_urls, mask_metadata_value};
use crate::output::MatchRecord;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ScanResult {
    pub(crate) failure: Option<MatchRecord>,
    pub(crate) text_file_count: usize,
}

pub(crate) fn scan_files(
    root: &Path,
    entries: &[FileEntry],
    configured: &ConfiguredRepository,
    scope: &str,
) -> Result<ScanResult, String> {
    let mut text_file_count = 0;
    for entry in entries {
        let absolute = if entry.path.is_absolute() {
            entry.path.clone()
        } else {
            root.join(&entry.path)
        };
        let relative = relative_display(root, &absolute)?;
        if let Some(failure) = scan_path(&relative, &format!("{scope}-path")) {
            return Ok(ScanResult {
                failure: Some(failure),
                text_file_count,
            });
        }
        if !entry.is_file {
            continue;
        }
        let bytes = fs::read(&absolute).map_err(|error| error.to_string())?;
        let Some(text) = decode_utf8(&bytes) else {
            continue;
        };
        text_file_count += 1;
        if let Some(failure) = scan_text(text, &relative, configured, &format!("{scope}-content")) {
            return Ok(ScanResult {
                failure: Some(failure),
                text_file_count,
            });
        }
    }
    Ok(ScanResult {
        failure: None,
        text_file_count,
    })
}

pub(crate) fn scan_metadata(
    entries: &[String],
    configured: &ConfiguredRepository,
) -> Result<ScanResult, String> {
    for entry in entries {
        let Some(separator) = entry.find('=') else {
            return Err("metadata must use key=value".to_owned());
        };
        if separator == 0 {
            return Err("metadata must use key=value".to_owned());
        }
        let key = &entry[..separator];
        let value = &entry[separator + 1..];
        let masked = mask_metadata_value(key, value, configured);
        if let Some(marker) = first_forbidden_match(&masked) {
            return Ok(ScanResult {
                failure: Some(MatchRecord::metadata(key, marker)),
                text_file_count: 0,
            });
        }
    }
    Ok(ScanResult::default())
}

fn scan_path(path: &str, kind: &str) -> Option<MatchRecord> {
    let normalized = path.replace('\\', "/");
    first_forbidden_match(&normalized).map(|marker| MatchRecord::path(kind, path, marker))
}

fn scan_text(
    text: &str,
    path: &str,
    configured: &ConfiguredRepository,
    kind: &str,
) -> Option<MatchRecord> {
    let masked = mask_canonical_repository_urls(text, configured);
    first_forbidden_match(&masked).map(|marker| MatchRecord::path(kind, path, marker))
}

fn decode_utf8(bytes: &[u8]) -> Option<&str> {
    if bytes.contains(&0) {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

fn relative_display(root: &Path, absolute: &Path) -> Result<String, String> {
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| "scan path escaped root".to_owned())?;
    if relative.as_os_str().is_empty() {
        return Ok(".".to_owned());
    }
    Ok(path_to_string(relative))
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::inventory::{filesystem_entries, resolve_inside_root};
    use crate::output::PathSurface;
    use crate::test_support::TempDir;

    fn marker_base() -> String {
        [107_u8, 105, 110, 101, 116, 105, 107]
            .into_iter()
            .map(char::from)
            .collect()
    }

    fn configured() -> ConfiguredRepository {
        ConfiguredRepository::parse(Some(&format!("{}-gg/stern", marker_base()))).unwrap()
    }

    fn write(path: &Path, bytes: impl AsRef<[u8]>) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    fn scan_fixture(fixture: &TempDir, start: &str) -> Result<ScanResult, String> {
        let start = resolve_inside_root(fixture.path(), start)?;
        let entries = filesystem_entries(fixture.path(), &start)?;
        scan_files(fixture.path(), &entries, &configured(), "filesystem")
    }

    #[test]
    fn clean_subtree_counts_only_decoded_text_files() {
        let fixture = TempDir::new("clean-subtree");
        write(&fixture.path().join("sub/a.txt"), "Stern — UTF-8");
        write(&fixture.path().join("sub/b.bin"), b"binary\0content");
        write(&fixture.path().join("sub/c.bin"), [0xff, 0xfe]);
        let result = scan_fixture(&fixture, "sub").unwrap();
        assert_eq!(
            result,
            ScanResult {
                failure: None,
                text_file_count: 1
            }
        );
    }

    #[test]
    fn first_content_failure_has_deterministic_order_and_count() {
        let fixture = TempDir::new("content-order");
        write(&fixture.path().join("a-clean.txt"), "clean");
        write(
            &fixture.path().join("b-fail.txt"),
            format!("prefix-{}-suffix", marker_base()),
        );
        write(&fixture.path().join("c-fail.txt"), marker_base());
        let result = scan_fixture(&fixture, ".").unwrap();
        assert_eq!(result.text_file_count, 2);
        match result.failure.unwrap() {
            MatchRecord::Path {
                kind,
                surface: PathSurface::Path(path),
                marker,
            } => {
                assert_eq!(kind, "filesystem-content");
                assert_eq!(path, "b-fail.txt");
                assert_eq!(marker.normalized_length, 7);
            }
            other => panic!("unexpected record: {other:?}"),
        }
    }

    #[test]
    fn path_failure_precedes_content_and_hashes_the_path() {
        let fixture = TempDir::new("path-failure");
        let name = format!("{}.txt", marker_base());
        write(&fixture.path().join(&name), "clean");
        let result = scan_fixture(&fixture, ".").unwrap();
        assert_eq!(result.text_file_count, 0);
        match result.failure.unwrap() {
            MatchRecord::Path {
                kind,
                surface: PathSurface::Sha256(_),
                ..
            } => {
                assert_eq!(kind, "filesystem-path");
            }
            other => panic!("unexpected record: {other:?}"),
        }
    }

    #[test]
    fn canonical_urls_are_masked_but_other_occurrences_fail() {
        let fixture = TempDir::new("url-mask");
        let configured = configured();
        write(
            &fixture.path().join("canonical.txt"),
            format!("https://github.com/{}/issues/1", configured.repository),
        );
        assert!(
            scan_fixture(&fixture, "canonical.txt")
                .unwrap()
                .failure
                .is_none()
        );
        write(
            &fixture.path().join("other.txt"),
            format!("https://example.com/{}", configured.repository),
        );
        assert!(
            scan_fixture(&fixture, "other.txt")
                .unwrap()
                .failure
                .is_some()
        );
    }

    #[test]
    fn metadata_masks_exact_fields_and_rejects_other_values() {
        let configured = configured();
        let passing = vec![
            format!("repository={}", configured.repository),
            format!("origin=git@github.com:{}.git", configured.repository),
            "empty=".to_owned(),
        ];
        assert_eq!(
            scan_metadata(&passing, &configured).unwrap(),
            ScanResult::default()
        );

        let failure = scan_metadata(&[format!("owner={}", configured.owner)], &configured).unwrap();
        assert!(matches!(
            failure.failure,
            Some(MatchRecord::Metadata { .. })
        ));
        assert_eq!(
            scan_metadata(&["missing-separator".to_owned()], &configured),
            Err("metadata must use key=value".to_owned())
        );
        assert_eq!(
            scan_metadata(&["=missing-key".to_owned()], &configured),
            Err("metadata must use key=value".to_owned())
        );
    }

    #[test]
    fn missing_files_are_execution_errors() {
        let fixture = TempDir::new("missing");
        assert!(scan_fixture(&fixture, "missing.txt").is_err());
    }

    #[test]
    fn a_directory_entry_is_scanned_as_a_path_but_not_content() {
        let fixture = TempDir::new("directory");
        fs::create_dir(fixture.path().join("empty")).unwrap();
        let result = scan_fixture(&fixture, "empty").unwrap();
        assert_eq!(result.text_file_count, 0);
        assert!(result.failure.is_none());
    }

    #[test]
    fn decoding_rejects_nul_and_malformed_utf8() {
        assert!(decode_utf8(b"ok").is_some());
        assert!(decode_utf8(b"nul\0byte").is_none());
        assert!(decode_utf8(&[0xff]).is_none());
    }

    #[test]
    fn relative_root_entry_is_a_dot() {
        let fixture = TempDir::new("root-display");
        assert_eq!(
            relative_display(fixture.path(), fixture.path()).unwrap(),
            "."
        );
        assert!(relative_display(fixture.path(), &PathBuf::from("outside")).is_err());
    }
}
