use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::marker::first_forbidden_match;
use crate::mask::{ConfiguredRepository, mask_canonical_repository_urls, mask_metadata_value};
use crate::output::MatchRecord;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PackageIdentity {
    name: String,
    crate_name: String,
    path: String,
}

pub(crate) fn run(root: &Path) -> Result<(), String> {
    let base = [107_u8, 105, 110, 101, 116, 105, 107]
        .into_iter()
        .map(char::from)
        .collect::<String>();
    let suffix = [117_u8, 105]
        .into_iter()
        .map(char::from)
        .collect::<String>();
    let configured = ConfiguredRepository::parse(Some(&format!("{base}-gg/stern")))?;

    ensure(
        first_forbidden_match(&base.to_ascii_uppercase())
            .is_some_and(|marker| marker.normalized_length == 7),
    )?;
    ensure(
        first_forbidden_match(&format!("{base}-{suffix}"))
            .is_some_and(|marker| marker.normalized_length == 7),
    )?;
    let marker = first_forbidden_match(&base).ok_or_else(self_test_failed)?;
    ensure(!format!("{:?}", MatchRecord::path("test", "clean", marker)).contains(&base))?;
    ensure(
        first_forbidden_match(&mask_canonical_repository_urls(
            &format!("https://github.com/{}", configured.repository),
            &configured,
        ))
        .is_none(),
    )?;
    ensure(
        first_forbidden_match(&mask_canonical_repository_urls(
            &format!("https://github.com/{}/issues/1", configured.repository),
            &configured,
        ))
        .is_none(),
    )?;
    ensure(
        first_forbidden_match(&mask_canonical_repository_urls(
            &format!("https://example.com/{}", configured.repository),
            &configured,
        ))
        .is_some(),
    )?;
    ensure(
        first_forbidden_match(&format!("fixtures/{}.txt", base.to_ascii_uppercase())).is_some(),
    )?;
    ensure(first_forbidden_match(&format!("prefix-{base}-suffix")).is_some())?;
    ensure(first_forbidden_match("Stern — UTF-8").is_none())?;
    ensure(
        !mask_metadata_value("repository", &configured.repository, &configured)
            .contains(&configured.owner),
    )?;
    ensure(
        first_forbidden_match(&mask_metadata_value(
            "owner",
            &configured.owner,
            &configured,
        ))
        .is_some(),
    )?;
    run_inventory_test(root)
}

fn ensure(condition: bool) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(self_test_failed())
    }
}

fn self_test_failed() -> String {
    "identity scanner self-test failed".to_owned()
}

fn run_inventory_test(root: &Path) -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let metadata: Value =
        serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(self_test_failed)?;
    let mut actual = packages
        .iter()
        .map(|package| package_identity(root, package))
        .collect::<Result<Vec<_>, _>>()?;
    actual.sort_by(|left, right| left.path.cmp(&right.path));

    let mut expected = expected_packages();
    expected.sort_by(|left, right| left.path.cmp(&right.path));
    if actual == expected {
        Ok(())
    } else {
        Err("workspace identity inventory mismatch".to_owned())
    }
}

fn package_identity(root: &Path, package: &Value) -> Result<PackageIdentity, String> {
    let name = package
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(self_test_failed)?
        .to_owned();
    let targets = package
        .get("targets")
        .and_then(Value::as_array)
        .ok_or_else(self_test_failed)?;
    let target = targets
        .iter()
        .find(|target| {
            target
                .get("kind")
                .and_then(Value::as_array)
                .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("lib")))
        })
        .or_else(|| targets.first())
        .ok_or_else(self_test_failed)?;
    let crate_name = target
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(self_test_failed)?
        .to_owned();
    let manifest_path = package
        .get("manifest_path")
        .and_then(Value::as_str)
        .ok_or_else(self_test_failed)?;
    let package_path = PathBuf::from(manifest_path)
        .parent()
        .ok_or_else(self_test_failed)?
        .strip_prefix(root)
        .map_err(|_| self_test_failed())?
        .to_string_lossy()
        .replace('\\', "/");
    Ok(PackageIdentity {
        name,
        crate_name,
        path: package_path,
    })
}

fn expected_packages() -> Vec<PackageIdentity> {
    [
        ("stern", "stern", "crates/stern"),
        ("stern-core", "stern_core", "crates/stern-core"),
        (
            "stern-icon-atlas",
            "stern_icon_atlas",
            "crates/stern-icon-atlas",
        ),
        (
            "stern-identity-scan",
            "stern_identity_scan",
            "crates/stern-identity-scan",
        ),
        (
            "stern-icons-phosphor",
            "stern_icons_phosphor",
            "crates/stern-icons-phosphor",
        ),
        ("stern-render", "stern_render", "crates/stern-render"),
        ("stern-text", "stern_text", "crates/stern-text"),
        ("stern-vello", "stern_vello", "crates/stern-vello"),
        (
            "stern-vello-winit",
            "stern_vello_winit",
            "crates/stern-vello-winit",
        ),
        ("stern-widgets", "stern_widgets", "crates/stern-widgets"),
        ("stern-winit", "stern_winit", "crates/stern-winit"),
        ("stern-demo", "stern_demo", "apps/stern-demo"),
    ]
    .into_iter()
    .map(|(name, crate_name, path)| PackageIdentity {
        name: name.to_owned(),
        crate_name: crate_name.to_owned(),
        path: path.to_owned(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_inventory_contains_only_expected_packages() {
        let root =
            crate::inventory::real_root(Path::new(env!("CARGO_MANIFEST_DIR")), "../..").unwrap();
        run_inventory_test(&root).unwrap();
    }

    #[test]
    fn self_test_keeps_its_non_disclosing_contract() {
        let root =
            crate::inventory::real_root(Path::new(env!("CARGO_MANIFEST_DIR")), "../..").unwrap();
        run(&root).unwrap();
    }
}
