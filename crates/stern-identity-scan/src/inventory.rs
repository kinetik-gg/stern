use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::collation::PinnedCollator;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FileEntry {
    pub(crate) path: PathBuf,
    pub(crate) is_file: bool,
}

pub(crate) fn real_root(cwd: &Path, requested: &str) -> Result<PathBuf, String> {
    let requested = Path::new(requested);
    let unresolved = if requested.is_absolute() {
        requested.to_owned()
    } else {
        cwd.join(requested)
    };
    fs::canonicalize(unresolved)
        .map(normalize_windows_verbatim)
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
fn normalize_windows_verbatim(path: PathBuf) -> PathBuf {
    let display = path.to_string_lossy();
    if let Some(unc) = display.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{unc}"));
    }
    if let Some(local) = display.strip_prefix(r"\\?\") {
        return PathBuf::from(local);
    }
    drop(display);
    path
}

#[cfg(not(windows))]
fn normalize_windows_verbatim(path: PathBuf) -> PathBuf {
    path
}

pub(crate) fn resolve_inside_root(root: &Path, requested: &str) -> Result<PathBuf, String> {
    let resolved = node_style_join(root, requested)?;
    reject_symlink_ancestors(root, &resolved)?;
    Ok(resolved)
}

fn node_style_join(root: &Path, requested: &str) -> Result<PathBuf, String> {
    let requested = requested.trim_start_matches(is_node_separator);
    if requested.is_empty() {
        return Ok(root.to_owned());
    }
    let mut segments = Vec::new();
    for segment in requested.split(is_node_separator) {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return Err("scan path escaped root".to_owned());
                }
            }
            _ => segments.push(segment),
        }
    }
    let mut joined = root.to_string_lossy().into_owned();
    for segment in segments {
        joined.push(std::path::MAIN_SEPARATOR);
        joined.push_str(segment);
    }
    Ok(PathBuf::from(joined))
}

#[cfg(windows)]
fn is_node_separator(character: char) -> bool {
    matches!(character, '/' | '\\')
}

#[cfg(not(windows))]
fn is_node_separator(character: char) -> bool {
    character == '/'
}

fn reject_symlink_ancestors(root: &Path, target: &Path) -> Result<(), String> {
    if !target.starts_with(root) {
        return Err("scan path escaped root".to_owned());
    }
    if target == root {
        return Ok(());
    }
    let mut ancestor = target.parent();
    while let Some(path) = ancestor {
        if path == root {
            return Ok(());
        }
        if !path.starts_with(root) {
            return Err("scan path escaped root".to_owned());
        }
        if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err("scan path escaped root".to_owned());
        }
        ancestor = path.parent();
    }
    Err("scan path escaped root".to_owned())
}

pub(crate) fn effective_tracked_files(root: &Path) -> Result<Vec<FileEntry>, String> {
    let output = Command::new("git")
        .args(["ls-files", "-co", "--exclude-standard", "-z"])
        .current_dir(root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let message = if stderr.is_empty() {
            "Command failed: git ls-files -co --exclude-standard -z".to_owned()
        } else {
            format!("Command failed: git ls-files -co --exclude-standard -z\n{stderr}")
        };
        return Err(message);
    }

    let decoded = String::from_utf8_lossy(&output.stdout);
    let mut paths = decoded
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    PinnedCollator::new()?.sort_strings(&mut paths);
    let mut entries = Vec::new();
    for path in paths {
        let absolute = root.join(&path);
        if fs::symlink_metadata(&absolute).is_ok_and(|metadata| metadata.is_file()) {
            entries.push(FileEntry {
                path: PathBuf::from(path),
                is_file: true,
            });
        }
    }
    Ok(entries)
}

pub(crate) fn filesystem_entries(root: &Path, start: &Path) -> Result<Vec<FileEntry>, String> {
    if !start.starts_with(root) {
        return Err("scan path escaped root".to_owned());
    }
    let mut entries = Vec::new();
    let collator = PinnedCollator::new()?;
    visit(start, &mut entries, &collator)?;
    Ok(entries)
}

fn visit(
    path: &Path,
    entries: &mut Vec<FileEntry>,
    collator: &PinnedCollator,
) -> Result<(), String> {
    let metadata = lstat(path)?;
    entries.push(FileEntry {
        path: path.to_owned(),
        is_file: metadata.is_file(),
    });
    if !metadata.is_dir() {
        return Ok(());
    }

    let mut children = fs::read_dir(path)
        .map_err(|error| error.to_string())?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    collator.sort_paths_by_name(&mut children);
    for child in children {
        visit(&child, entries, collator)?;
    }
    Ok(())
}

fn lstat(path: &Path) -> Result<fs::Metadata, String> {
    fs::symlink_metadata(path).map_err(|error| {
        if is_missing_lstat_error(&error) {
            format!(
                "ENOENT: no such file or directory, lstat '{}'",
                path.display()
            )
        } else {
            error.to_string()
        }
    })
}

fn is_missing_lstat_error(error: &std::io::Error) -> bool {
    if error.kind() == std::io::ErrorKind::NotFound {
        return true;
    }
    #[cfg(windows)]
    return matches!(error.raw_os_error(), Some(2 | 3 | 123));
    #[cfg(not(windows))]
    false
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;
    use crate::test_support::TempDir;

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, text).unwrap();
    }

    #[test]
    fn filesystem_walk_includes_start_and_sorts_names() {
        let fixture = TempDir::new("filesystem-order");
        write(&fixture.path().join("b.txt"), "b");
        write(&fixture.path().join("a/2.txt"), "2");
        write(&fixture.path().join("a/1.txt"), "1");
        let entries = filesystem_entries(fixture.path(), fixture.path()).unwrap();
        let relative = entries
            .iter()
            .map(|entry| entry.path.strip_prefix(fixture.path()).unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            relative,
            [
                PathBuf::from(""),
                PathBuf::from("a"),
                PathBuf::from("a/1.txt"),
                PathBuf::from("a/2.txt"),
                PathBuf::from("b.txt")
            ]
        );
        assert!(!entries[0].is_file);
    }

    #[test]
    fn lexical_root_escapes_fail_without_requiring_existing_descendants() {
        let fixture = TempDir::new("root-escape");
        assert_eq!(
            resolve_inside_root(fixture.path(), "../outside"),
            Err("scan path escaped root".to_owned())
        );
        assert_eq!(
            resolve_inside_root(fixture.path(), "missing/child").unwrap(),
            fixture.path().join("missing/child")
        );
    }

    #[test]
    fn leading_platform_separators_stay_root_relative() {
        let fixture = TempDir::new("rooted-path");
        write(&fixture.path().join("file.txt"), "clean");
        assert_eq!(
            resolve_inside_root(fixture.path(), "/file.txt").unwrap(),
            fixture.path().join("file.txt")
        );
        #[cfg(windows)]
        assert_eq!(
            resolve_inside_root(fixture.path(), "\\file.txt").unwrap(),
            fixture.path().join("file.txt")
        );
    }

    #[cfg(windows)]
    #[test]
    fn drive_and_unc_forms_are_joined_beneath_the_root() {
        let fixture = TempDir::new("windows-root-forms");
        for path in [
            "C:/missing.txt",
            "C:\\missing.txt",
            "//server/share/missing.txt",
            "\\\\server\\share\\missing.txt",
        ] {
            assert!(
                resolve_inside_root(fixture.path(), path)
                    .unwrap()
                    .starts_with(fixture.path())
            );
        }
    }

    #[test]
    fn tracked_inventory_includes_effective_files_only() {
        let fixture = TempDir::new("tracked");
        let git = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(fixture.path())
                .status()
                .unwrap();
            assert!(status.success());
        };
        git(&["init", "-q"]);
        write(&fixture.path().join(".gitignore"), "ignored.txt\n");
        write(&fixture.path().join("tracked.txt"), "tracked");
        write(&fixture.path().join("staged.txt"), "staged");
        write(&fixture.path().join("deleted.txt"), "deleted");
        write(&fixture.path().join("untracked.txt"), "untracked");
        write(&fixture.path().join("folder/nested.txt"), "nested");
        write(&fixture.path().join("ignored.txt"), "ignored");
        fs::create_dir(fixture.path().join("empty-directory")).unwrap();
        git(&["add", "tracked.txt"]);
        git(&[
            "-c",
            "user.name=Stern Tests",
            "-c",
            "user.email=tests@invalid.example",
            "commit",
            "-qm",
            "tracked fixture",
        ]);
        git(&["add", "staged.txt", "deleted.txt"]);
        fs::remove_file(fixture.path().join("deleted.txt")).unwrap();

        let actual = effective_tracked_files(fixture.path())
            .unwrap()
            .into_iter()
            .map(|entry| entry.path)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            [
                PathBuf::from(".gitignore"),
                PathBuf::from("folder/nested.txt"),
                PathBuf::from("staged.txt"),
                PathBuf::from("tracked.txt"),
                PathBuf::from("untracked.txt"),
            ]
        );
    }

    #[test]
    fn non_repository_git_failure_is_reported() {
        let fixture = TempDir::new("not-git");
        let error = effective_tracked_files(fixture.path()).unwrap_err();
        assert!(error.starts_with("Command failed: git ls-files"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_directories_are_not_followed() {
        use std::os::unix::fs::symlink;

        let fixture = TempDir::new("symlink");
        let outside = TempDir::new("symlink-outside");
        write(&outside.path().join("secret.txt"), "secret");
        symlink(outside.path(), fixture.path().join("link")).unwrap();
        let entries = filesystem_entries(fixture.path(), fixture.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(!entries[1].is_file);
        assert_eq!(
            resolve_inside_root(fixture.path(), "link/secret.txt"),
            Err("scan path escaped root".to_owned())
        );
    }

    #[cfg(windows)]
    #[test]
    fn symlinked_directories_are_not_followed() {
        use std::io::ErrorKind;
        use std::os::windows::fs::symlink_dir;

        let fixture = TempDir::new("symlink");
        let outside = TempDir::new("symlink-outside");
        write(&outside.path().join("secret.txt"), "secret");
        match symlink_dir(outside.path(), fixture.path().join("link")) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::PermissionDenied => return,
            Err(error) => panic!("failed to create symlink fixture: {error}"),
        }
        let entries = filesystem_entries(fixture.path(), fixture.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(!entries[1].is_file);
        assert_eq!(
            resolve_inside_root(fixture.path(), "link/secret.txt"),
            Err("scan path escaped root".to_owned())
        );
    }
}
