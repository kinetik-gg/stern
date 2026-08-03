use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use icu_collator::{CollatorBorrowed, CollatorPreferences, options::CollatorOptions};

/// Pinned replacement for the original runtime's default `localeCompare`.
///
/// The captured Node baseline resolves to `en-ID`. Its standard collation for
/// Stern's supported path fixture domain matches the ICU root collation. The
/// exact ICU4X 2.2.0 code and compiled data are locked in `Cargo.lock`, so path
/// ordering does not vary with the host locale. Byte ordering is used only as
/// a deterministic tie-breaker when Unicode collation considers strings equal.
pub(crate) struct PinnedCollator {
    collator: CollatorBorrowed<'static>,
}

impl PinnedCollator {
    pub(crate) fn new() -> Result<Self, String> {
        CollatorBorrowed::try_new(CollatorPreferences::default(), CollatorOptions::default())
            .map(|collator| Self { collator })
            .map_err(|error| error.to_string())
    }

    pub(crate) fn compare(&self, left: &str, right: &str) -> Ordering {
        self.collator
            .compare(left, right)
            .then_with(|| left.cmp(right))
    }

    pub(crate) fn sort_strings(&self, values: &mut [String]) {
        values.sort_by(|left, right| self.compare(left, right));
    }

    pub(crate) fn sort_paths_by_name(&self, values: &mut [PathBuf]) {
        values.sort_by(|left, right| {
            let left = file_name(left);
            let right = file_name(right);
            self.compare(&left, &right)
        });
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_captured_locale_compare_for_punctuation_case_accents_and_non_ascii() {
        let collator = PinnedCollator::new().unwrap();
        let mut names = [
            "-x.txt", "_x.txt", "10.txt", "2.txt", "a.txt", "A.txt", "á.txt", "ä.txt", "b.txt",
            "é.txt", "z.txt", "中.txt",
        ]
        .map(str::to_owned);
        collator.sort_strings(&mut names);
        assert_eq!(
            names,
            [
                "_x.txt", "-x.txt", "10.txt", "2.txt", "a.txt", "A.txt", "á.txt", "ä.txt", "b.txt",
                "é.txt", "z.txt", "中.txt",
            ]
        );
    }

    #[test]
    fn matches_captured_locale_compare_for_nested_paths() {
        let collator = PinnedCollator::new().unwrap();
        let mut paths = [
            "z/file.txt",
            "a-z/file.txt",
            "a_z/file.txt",
            "a/file.txt",
            "A/file.txt",
            "á/file.txt",
        ]
        .map(str::to_owned);
        collator.sort_strings(&mut paths);
        assert_eq!(
            paths,
            [
                "a_z/file.txt",
                "a-z/file.txt",
                "a/file.txt",
                "A/file.txt",
                "á/file.txt",
                "z/file.txt",
            ]
        );
    }
}
