use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSnapshotArtifacts {
    pub expected: PathBuf,
    pub actual: PathBuf,
    pub diff: PathBuf,
}

impl CommandSnapshotArtifacts {
    fn for_snapshot(snapshot_name: &str) -> Self {
        let dir = command_snapshot_root().join(sanitize_snapshot_name(snapshot_name));

        Self {
            expected: dir.join("expected.txt"),
            actual: dir.join("actual.txt"),
            diff: dir.join("diff.txt"),
        }
    }
}

pub fn command_snapshot_root() -> PathBuf {
    workspace_root()
        .join("target")
        .join("kinetik-ui-artifacts")
        .join("kinetik-ui-vello")
        .join("command-snapshots")
}

pub fn command_snapshot_artifact_paths(snapshot_name: &str) -> CommandSnapshotArtifacts {
    CommandSnapshotArtifacts::for_snapshot(snapshot_name)
}

pub fn assert_command_snapshot(snapshot_name: &str, expected: &str, actual: &str) {
    if expected == actual {
        return;
    }

    let artifacts = emit_command_snapshot_artifacts(snapshot_name, expected, actual)
        .expect("write Vello command snapshot mismatch artifacts");

    panic!(
        "Vello command snapshot `{snapshot_name}` did not match.\nexpected: {}\nactual: {}\ndiff: {}",
        artifacts.expected.display(),
        artifacts.actual.display(),
        artifacts.diff.display()
    );
}

pub fn emit_command_snapshot_artifacts(
    snapshot_name: &str,
    expected: &str,
    actual: &str,
) -> std::io::Result<CommandSnapshotArtifacts> {
    let artifacts = CommandSnapshotArtifacts::for_snapshot(snapshot_name);
    let dir = artifacts
        .expected
        .parent()
        .expect("artifact path has parent directory");

    fs::create_dir_all(dir)?;
    fs::write(&artifacts.expected, expected)?;
    fs::write(&artifacts.actual, actual)?;
    fs::write(&artifacts.diff, unified_line_diff(expected, actual))?;

    Ok(artifacts)
}

pub fn remove_command_snapshot_artifacts(snapshot_name: &str) -> std::io::Result<()> {
    let dir = CommandSnapshotArtifacts::for_snapshot(snapshot_name)
        .expected
        .parent()
        .expect("artifact path has parent directory")
        .to_owned();

    match fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn workspace_root() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    crate_dir
        .ancestors()
        .find(|path| path.join("Cargo.lock").is_file())
        .expect("workspace root contains Cargo.lock")
        .to_owned()
}

fn sanitize_snapshot_name(snapshot_name: &str) -> String {
    let mut sanitized = String::with_capacity(snapshot_name.len());

    for character in snapshot_name.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
            sanitized.push(character);
        } else {
            sanitized.push('_');
        }
    }

    sanitized.trim_matches('_').to_owned()
}

fn unified_line_diff(expected: &str, actual: &str) -> String {
    if expected == actual {
        return String::new();
    }

    let mut diff = String::from("--- expected\n+++ actual\n");
    let expected_lines: Vec<_> = expected.lines().collect();
    let actual_lines: Vec<_> = actual.lines().collect();
    let line_count = expected_lines.len().max(actual_lines.len());

    for index in 0..line_count {
        match (expected_lines.get(index), actual_lines.get(index)) {
            (Some(expected_line), Some(actual_line)) if expected_line == actual_line => {
                writeln!(diff, "  {expected_line}").expect("write diff line");
            }
            (Some(expected_line), Some(actual_line)) => {
                writeln!(diff, "- {expected_line}").expect("write expected diff line");
                writeln!(diff, "+ {actual_line}").expect("write actual diff line");
            }
            (Some(expected_line), None) => {
                writeln!(diff, "- {expected_line}").expect("write expected diff line");
            }
            (None, Some(actual_line)) => {
                writeln!(diff, "+ {actual_line}").expect("write actual diff line");
            }
            (None, None) => {}
        }
    }

    diff
}
