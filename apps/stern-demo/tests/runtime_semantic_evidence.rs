//! Focused generator and stale-evidence rejection checks.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn generator_records_honest_current_runtime_packet() {
    verify(&tracked_packet(), true);
    let evidence = temp("generated");
    generate(&evidence);
    assert_provisional(&evidence);
    let _ = fs::remove_file(evidence);
}

#[test]
fn verifier_rejects_stale_source_and_platform_evidence_tampering() {
    assert_mutation_rejected("record.source.tree = '0'.repeat(40);", "stale-source");
    assert_mutation_rejected(
        "record.platformEvidence.records.find(record => record.platform === 'linux').exitCode = 1;",
        "platform",
    );
}

#[test]
fn verifier_rejects_incomplete_component_claim() {
    assert_mutation_rejected(
        "record.runtime.components[0].status = 'notExecuted';",
        "component",
    );
}

#[test]
fn verifier_rejects_failed_journey_and_traversal_claims() {
    assert_mutation_rejected("record.runtime.journeys[0].status = 'failed';", "journey");
    assert_mutation_rejected("record.traversalTraces[0].status = 'failed';", "traversal");
}

#[test]
fn verifier_rejects_failed_owner_cleanup_claim() {
    assert_mutation_rejected(
        "record.focusRestorationTraces.find(trace => trace.interaction === 'focus-owner removal cleanup').restored = false;",
        "owner-cleanup",
    );
}

#[test]
fn verifier_rejects_component_workspace_membership_tampering() {
    assert_mutation_rejected(
        "record.runtime.components[0].workspaceIds = ['graph-workspace'];",
        "component-workspace",
    );
}

#[test]
fn verifier_rejects_journey_evidence_reference_tampering() {
    assert_mutation_rejected(
        "record.runtime.journeys[0].evidenceRefs = ['#/source'];",
        "journey-evidence-ref",
    );
}

#[test]
fn verifier_rejects_semantic_label_tampering() {
    assert_mutation_rejected(
        "record.semanticSnapshots[0].nodes.find(node => node.label).label = 'tampered';",
        "semantic-label",
    );
}

#[test]
fn verifier_rejects_traversal_focus_outcome_tampering() {
    assert_mutation_rejected(
        "record.traversalTraces[0].focusAfter = '0000000000000000';",
        "traversal-focus-after",
    );
}

#[test]
fn verifier_rejects_focus_owner_tampering() {
    assert_mutation_rejected(
        "record.focusRestorationTraces[0].focusOwner = '0000000000000000';",
        "focus-owner",
    );
}

#[test]
fn verifier_rejects_action_identity_and_state_tampering() {
    assert_mutation_rejected(
        "record.logs.actions[0].actionId = 'tampered'; record.logs.actions[0].stateAfter = 999;",
        "action-identity-state",
    );
}

#[test]
fn verifier_rejects_graph_edge_count_tampering() {
    assert_mutation_rejected(
        "record.logs.stateTransitions.find(log => log.id === 'graph-pointer-connection').edgesAfter = 999;",
        "graph-edges-after",
    );
}

#[test]
fn verifier_rejects_failure_preservation_and_feedback_tampering() {
    assert_mutation_rejected(
        "const failure = record.logs.failurePaths.find(log => log.id === 'color-style-save-failure'); failure.applicationStatePreserved = false; failure.semanticFeedback = false;",
        "failure-preservation-feedback",
    );
}

fn generate(path: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_runtime_semantic_evidence"))
        .args(["--output", path.to_str().unwrap(), "--source-ref", "HEAD"])
        .current_dir(repo_root())
        .status()
        .expect("run evidence generator");
    assert!(status.success());
}

fn verify(path: &Path, expected: bool) {
    let output = Command::new("node")
        .args([
            "apps/stern-demo/tools/check-runtime-semantic-evidence.mjs",
            "--evidence",
            path.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .expect("run evidence verifier");
    assert_eq!(
        output.status.success(),
        expected,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_mutation_rejected(mutation: &str, label: &str) {
    let evidence = temp(label);
    fs::copy(tracked_packet(), &evidence).expect("copy tracked evidence packet");
    mutate(&evidence, mutation);
    verify_integrity_rejected(&evidence);
    let _ = fs::remove_file(evidence);
}

fn verify_integrity_rejected(path: &Path) {
    let output = verifier(path);
    assert!(
        !output.status.success(),
        "tampered packet unexpectedly passed"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("packet integrity: SHA-256 mismatch"),
        "missing packet-integrity diagnostic: {stderr}"
    );
}

fn verifier(path: &Path) -> std::process::Output {
    Command::new("node")
        .args([
            "apps/stern-demo/tools/check-runtime-semantic-evidence.mjs",
            "--evidence",
            path.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .expect("run evidence verifier")
}

fn tracked_packet() -> PathBuf {
    repo_root().join("apps/stern-demo/tests/evidence/runtime-semantic-evidence.provisional.json")
}

fn assert_provisional(path: &Path) {
    let script = concat!(
        "const fs=require('fs');const r=JSON.parse(fs.readFileSync(process.argv[1],'utf8'));",
        "const passed=x=>x.filter(v=>v.status==='passed').length;",
        "const gate=id=>r.gates.find(v=>v.id===id).status;",
        "if(r.status!=='final'||r.runtime.components.length!==34||",
        "passed(r.runtime.components)!==34||r.runtime.journeys.length!==7||",
        "passed(r.runtime.journeys)!==7||r.semanticSnapshots.length!==2||",
        "!r.publicConsumerAudit.passed||gate('renderer-and-scale-quality')!=='passed'||",
        "gate('platform-integration')!=='passed'||r.knownGaps.length!==0)process.exit(1);",
    );
    let status = Command::new("node")
        .args(["-e", script, path.to_str().unwrap()])
        .status()
        .expect("inspect provisional evidence");
    assert!(status.success());
}

fn mutate(path: &Path, mutation: &str) {
    let script = format!(
        "const fs=require('fs');const path=process.argv[1];const record=JSON.parse(fs.readFileSync(path,'utf8'));{mutation}fs.writeFileSync(path,JSON.stringify(record));"
    );
    let status = Command::new("node")
        .args(["-e", &script, path.to_str().unwrap()])
        .status()
        .expect("tamper evidence fixture");
    assert!(status.success());
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "stern-runtime-semantic-evidence-{label}-{}.json",
        std::process::id()
    ))
}
