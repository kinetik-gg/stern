//! Offline release-fixture build and binary linkage inspection.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use stern_icon_atlas::{Catalog, Snapshot, VENDORED_ARCHIVE};

type DynResult<T> = Result<T, Box<dyn std::error::Error>>;

const BASELINE_MARKER: &[u8] = b"STERN_PHOSPHOR_2_1_1:regular:airplane";
const ADDED_MARKER: &[u8] = b"STERN_PHOSPHOR_2_1_1:regular:floppy-disk";
const SENTINEL_NAME: &str = "anchor";
const SENTINEL_MARKER: &[u8] = b"STERN_PHOSPHOR_2_1_1:regular:anchor";
const MAX_ONE_ICON_DELTA: u64 = 65_536;

pub(crate) fn check_workspace() -> DynResult<()> {
    let workspace = workspace_root();
    verify_sentinel_definition(&workspace)?;
    let linkage_target = workspace.join("target").join("linkage-check");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(&workspace)
        .env("CARGO_TARGET_DIR", &linkage_target)
        .args([
            "build",
            "--offline",
            "--release",
            "-p",
            "stern-icons-phosphor",
            "--example",
            "linkage-baseline",
            "--example",
            "linkage-with-one",
        ])
        .status()?;
    if !status.success() {
        return Err(format!("release fixture build failed with {status}").into());
    }
    let target = linkage_target.join("release").join("examples");
    let baseline = executable(&target, "linkage-baseline");
    let with_one = executable(&target, "linkage-with-one");
    let baseline_bytes = inspect_fixture(
        &baseline,
        &[BASELINE_MARKER],
        &[ADDED_MARKER, SENTINEL_MARKER],
    )?;
    let with_one_bytes = inspect_fixture(
        &with_one,
        &[BASELINE_MARKER, ADDED_MARKER],
        &[SENTINEL_MARKER],
    )?;
    run_fixture(&baseline, &[BASELINE_MARKER, b":62"])?;
    run_fixture(&with_one, &[BASELINE_MARKER, ADDED_MARKER, b":62", b":45"])?;
    let baseline_size = baseline_bytes.len() as u64;
    let with_one_size = with_one_bytes.len() as u64;
    let delta = baseline_size.abs_diff(with_one_size);
    if delta > MAX_ONE_ICON_DELTA {
        return Err(
            format!("one-icon release size delta {delta} exceeds {MAX_ONE_ICON_DELTA}").into(),
        );
    }
    println!(
        "linkage baseline: {} ({} bytes)",
        baseline.display(),
        baseline_size
    );
    println!(
        "linkage with one: {} ({} bytes)",
        with_one.display(),
        with_one_size
    );
    println!("one-icon absolute size delta: {delta} bytes (bound {MAX_ONE_ICON_DELTA})");
    println!("workspace release profile: thin LTO");
    println!("referenced identity markers and traversed graphic/path element counts: present");
    println!(
        "unrelated sentinel {}: absent",
        String::from_utf8_lossy(SENTINEL_MARKER)
    );
    Ok(())
}

fn verify_sentinel_definition(workspace: &Path) -> DynResult<()> {
    let snapshot = Snapshot::open(workspace.join(VENDORED_ARCHIVE))?;
    let catalog = Catalog::from_snapshot(&snapshot)?;
    if catalog
        .records
        .iter()
        .filter(|record| record.name == SENTINEL_NAME)
        .count()
        != 1
    {
        return Err(format!("linkage sentinel `{SENTINEL_NAME}` is not uniquely canonical").into());
    }

    let regular = workspace.join("crates/stern-icons-phosphor/src/generated/regular");
    let exact_marker = format!("\"{}\"", std::str::from_utf8(SENTINEL_MARKER)?);
    let mut marker_count = 0_usize;
    let mut constant_count = 0_usize;
    for source in rust_sources(&regular)? {
        marker_count += source.matches(&exact_marker).count();
        constant_count += source.matches("pub const ANCHOR: PhosphorIcon =").count();
    }
    if (marker_count, constant_count) != (1, 1) {
        return Err(format!(
            "generated regular sentinel must have one marker and one constant; found {marker_count} and {constant_count}"
        )
        .into());
    }
    Ok(())
}

fn rust_sources(directory: &Path) -> DynResult<Vec<String>> {
    let mut paths = fs::read_dir(directory)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(fs::read_to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("atlas crate must be inside workspace/crates")
        .to_path_buf()
}

fn executable(directory: &Path, stem: &str) -> PathBuf {
    directory.join(format!("{stem}{}", env::consts::EXE_SUFFIX))
}

fn inspect_fixture(path: &Path, required: &[&[u8]], forbidden: &[&[u8]]) -> DynResult<Vec<u8>> {
    let bytes = fs::read(path)?;
    for marker in required {
        if !contains(&bytes, marker) {
            return Err(format!(
                "{} lacks required marker {}",
                path.display(),
                String::from_utf8_lossy(marker)
            )
            .into());
        }
    }
    for marker in forbidden {
        if contains(&bytes, marker) {
            return Err(format!(
                "{} unexpectedly retains marker {}",
                path.display(),
                String::from_utf8_lossy(marker)
            )
            .into());
        }
    }
    Ok(bytes)
}

fn run_fixture(path: &Path, required: &[&[u8]]) -> DynResult<()> {
    let output = Command::new(path).output()?;
    if !output.status.success() {
        return Err(format!("{} failed", path.display()).into());
    }
    for marker in required {
        if !contains(&output.stdout, marker) {
            return Err(format!(
                "{} did not exercise marker {} and its graphic data",
                path.display(),
                String::from_utf8_lossy(marker)
            )
            .into());
        }
    }
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::{contains, verify_sentinel_definition, workspace_root};

    #[test]
    fn binary_marker_scan_is_exact() {
        assert!(contains(b"prefix-marker-suffix", b"marker"));
        assert!(!contains(b"prefix-marker-suffix", b"unrelated"));
        assert!(!contains(b"anything", b""));
    }

    #[test]
    fn selected_sentinel_exists_in_catalog_and_generated_source() {
        verify_sentinel_definition(&workspace_root()).expect("real generated sentinel");
    }
}
