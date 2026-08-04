//! Machine validation for the hand-curated parity claims ledger.
//!
//! `conformance/claims.json` is stern's only conformance claim surface
//! against the design system's requirement index. The design-system rule is
//! "a claim without a validating record is not a claim", so this contract
//! fails the workspace test run whenever a claim names a requirement that
//! does not exist, cites a test that does not exist in this repository, or
//! carries a status the current evidence cannot support.
//!
//! Checks that need the read-only `../stern-design-system` checkout (the
//! requirement-index membership and schema drift checks) skip with a note
//! when the sibling checkout is absent, matching the theme token drift test
//! in `crates/stern-core/src/theme/tests.rs`. Set `STERN_DESIGN_SYSTEM_DIR`
//! to point at a design-system checkout that is not a workspace sibling
//! (for example when running from a git worktree).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Statuses the vendored parity-evidence schema declares, in schema order.
const SCHEMA_STATUSES: [&str; 3] = ["unverified", "partial", "verified"];

/// The only status stern's current evidence supports: model-layer automated
/// tests exist, but no specimen, browser/Vello baseline, scale, platform,
/// or review-record evidence does.
const SUPPORTED_STATUS: &str = "partial";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn design_system_root() -> PathBuf {
    std::env::var_os("STERN_DESIGN_SYSTEM_DIR").map_or_else(
        || workspace_root().join("../stern-design-system"),
        PathBuf::from,
    )
}

fn read_json(path: &Path) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parse {} as JSON: {error}", path.display()))
}

fn claims() -> Vec<Value> {
    let path = workspace_root().join("conformance/claims.json");
    let manifest = read_json(&path);
    let claims = manifest
        .as_array()
        .expect("conformance/claims.json is a top-level array of claims")
        .clone();
    assert!(!claims.is_empty(), "claims manifest must not be empty");
    claims
}

fn claim_str<'a>(claim: &'a Value, key: &str) -> &'a str {
    claim
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("claim {claim} carries a string `{key}`"))
}

fn requirement_id_is_wellformed(id: &str) -> bool {
    let Some(tail) = id.strip_prefix("STERN-") else {
        return false;
    };
    let Some((family, number)) = tail.split_once('-') else {
        return false;
    };
    !family.is_empty()
        && family
            .chars()
            .all(|character| character.is_ascii_uppercase())
        && number.len() == 3
        && number.chars().all(|character| character.is_ascii_digit())
}

fn test_reference_path_is_wellformed(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "..")
        && Path::new(path)
            .extension()
            .is_some_and(|extension| extension == "rs")
}

/// Validates one `path::test_fn` reference against the repository.
fn assert_test_reference_exists(root: &Path, id: &str, reference: &str) {
    let (path, test_fn) = reference.split_once("::").unwrap_or_else(|| {
        panic!("{id}: test reference `{reference}` must use the `path::test_fn` form")
    });
    assert!(
        test_reference_path_is_wellformed(path),
        "{id}: test path `{path}` must be a repo-relative forward-slash .rs path"
    );
    assert!(
        !test_fn.is_empty()
            && test_fn
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
        "{id}: `{test_fn}` is not a plausible test function name"
    );

    let file = root.join(path);
    assert!(
        file.is_file(),
        "{id}: cited test file `{path}` does not exist in this repository"
    );
    let source = fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("{id}: read cited test file `{path}`: {error}"));
    assert!(
        source.contains("#[test]"),
        "{id}: cited file `{path}` contains no #[test] functions"
    );
    assert!(
        source.contains(&format!("fn {test_fn}(")),
        "{id}: cited test `{test_fn}` does not exist in `{path}`"
    );
}

#[test]
fn claims_manifest_is_wellformed_and_references_real_tests() {
    let root = workspace_root();
    let claims = claims();

    let mut previous_id: Option<String> = None;
    for claim in &claims {
        let object = claim.as_object().expect("each claim is a JSON object");
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["notes", "requirementId", "status", "tests"],
            "each claim carries exactly requirementId, status, tests, and notes"
        );

        let id = claim_str(claim, "requirementId");
        assert!(
            requirement_id_is_wellformed(id),
            "`{id}` is not a STERN-<FAMILY>-<NNN> requirement id"
        );
        if let Some(previous) = &previous_id {
            assert!(
                previous.as_str() < id,
                "claims must be sorted by requirementId with no duplicates: \
                 `{id}` follows `{previous}`"
            );
        }
        previous_id = Some(id.to_owned());

        assert!(
            !claim_str(claim, "notes").trim().is_empty(),
            "{id}: notes must state what the cited tests prove"
        );

        let tests = claim
            .get("tests")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("{id}: `tests` must be an array"));
        assert!(
            !tests.is_empty(),
            "{id}: a claim without a validating record is not a claim; cite \
             at least one test or remove the claim"
        );
        let mut seen = Vec::new();
        for reference in tests {
            let reference = reference
                .as_str()
                .unwrap_or_else(|| panic!("{id}: test references must be strings"));
            assert!(
                !seen.contains(&reference),
                "{id}: duplicate test reference `{reference}`"
            );
            seen.push(reference);
            assert_test_reference_exists(&root, id, reference);
        }
    }
}

#[test]
fn claim_statuses_are_schema_statuses_and_stay_partial() {
    let schema = read_json(&workspace_root().join("conformance/parity-evidence.schema.json"));
    let schema_statuses: Vec<&str> =
        schema["properties"]["evidence"]["items"]["properties"]["status"]["enum"]
            .as_array()
            .expect("vendored parity-evidence schema declares an evidence status enum")
            .iter()
            .map(|status| status.as_str().expect("status enum entries are strings"))
            .collect();
    assert_eq!(
        schema_statuses, SCHEMA_STATUSES,
        "vendored schema status vocabulary changed; revisit the claim policy"
    );

    for claim in claims() {
        let id = claim_str(&claim, "requirementId");
        let status = claim_str(&claim, "status");
        assert!(
            schema_statuses.contains(&status),
            "{id}: status `{status}` is not a parity-evidence schema status"
        );
        assert_eq!(
            status, SUPPORTED_STATUS,
            "{id}: stern has model-layer automated evidence only; no claim \
             can exceed `{SUPPORTED_STATUS}` until specimen, baseline, \
             scale, platform, and review evidence exists \
             (see conformance/README.md)"
        );
    }
}

#[test]
fn claims_match_design_system_requirement_index() {
    let index_path = design_system_root().join("generated/requirement-index.json");
    if !index_path.is_file() {
        eprintln!(
            "skipping requirement-index membership check: {} is not present",
            index_path.display()
        );
        return;
    }

    let index = read_json(&index_path);
    let requirements = index["requirements"]
        .as_array()
        .expect("requirement-index.json carries a requirements array");
    let sources: std::collections::BTreeMap<&str, &str> = requirements
        .iter()
        .map(|requirement| {
            (
                requirement["requirementId"]
                    .as_str()
                    .expect("requirement entries carry a requirementId"),
                requirement["source"]
                    .as_str()
                    .expect("requirement entries carry a source"),
            )
        })
        .collect();

    for claim in claims() {
        let id = claim_str(&claim, "requirementId");
        let source = sources.get(id).unwrap_or_else(|| {
            panic!("{id}: not a requirement in the design system's requirement-index.json")
        });
        assert!(
            source.starts_with("src/foundations/") || source.starts_with("src/behaviors/"),
            "{id}: source `{source}` is outside the foundations/behaviors \
             claim scope (see conformance/README.md)"
        );
        assert_ne!(
            *source, "src/foundations/accessibility.md",
            "{id}: accessibility requirements are excluded until an OS \
             accessibility bridge exists (see conformance/README.md)"
        );
    }
}

#[test]
fn vendored_evidence_schema_matches_design_system_source() {
    let upstream_path = design_system_root().join("schemas/parity-evidence.schema.json");
    let Ok(upstream) = fs::read_to_string(&upstream_path) else {
        eprintln!(
            "skipping parity-evidence schema drift check: {} is not present",
            upstream_path.display()
        );
        return;
    };
    let vendored =
        fs::read_to_string(workspace_root().join("conformance/parity-evidence.schema.json"))
            .expect("vendored parity-evidence.schema.json is readable");

    assert_eq!(
        vendored.replace("\r\n", "\n"),
        upstream.replace("\r\n", "\n"),
        "parity-evidence schema drifted: re-vendor \
         stern-design-system/schemas/parity-evidence.schema.json into \
         conformance/parity-evidence.schema.json (provenance is recorded in \
         conformance/README.md)"
    );
}
