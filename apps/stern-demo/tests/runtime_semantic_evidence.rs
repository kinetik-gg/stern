//! Focused generator and stale-evidence rejection checks.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const CANONICAL_SOURCE: &str = "50edc219ae5d013c242129adf2ec7a25942f5c28";
const CANONICAL_TREE: &str = "6c64e4dffe649a1b652cd5b0d15416e254c57b8c";
const RETIRED_TOPIC_SOURCE: &str = "6a677f098a463fb89e7fc727f28de50d65500cd0";
const RENDERER_CANONICAL_SOURCE: &str = "e7b46bb145b01c0b6e4ba570a628cb9fa2cc30f0";
const RENDERER_CANONICAL_TREE: &str = "a29fcf93865b9b797e82683cc395631b8d49e308";
const RETIRED_RENDERER_SOURCE: &str = "8aaa685e541d6b21edcee50d58e98dffefa62737";

#[test]
fn generator_records_honest_current_runtime_packet() {
    verify(&tracked_packet(), true);
    let evidence = temp("generated");
    generate(&evidence);
    assert_provisional(&evidence);
    let _ = fs::remove_file(evidence);
}

#[test]
fn tracked_packet_binds_resolvable_canonical_source_only() {
    let packet = fs::read_to_string(tracked_packet()).expect("read tracked evidence packet");
    assert!(packet.contains(CANONICAL_SOURCE));
    assert!(packet.contains(CANONICAL_TREE));
    assert!(!packet.contains(RETIRED_TOPIC_SOURCE));
    let commit = git(&["rev-parse", &format!("{CANONICAL_SOURCE}^{{commit}}")]);
    let tree = git(&["rev-parse", &format!("{CANONICAL_SOURCE}^{{tree}}")]);
    assert_eq!(commit, CANONICAL_SOURCE);
    assert_eq!(tree, CANONICAL_TREE);
}

#[test]
fn renderer_manifest_binds_resolvable_canonical_source_only() {
    let manifest = fs::read_to_string(renderer_manifest()).expect("read renderer manifest");
    assert!(manifest.contains(RENDERER_CANONICAL_SOURCE));
    assert!(manifest.contains(RENDERER_CANONICAL_TREE));
    assert!(!manifest.contains(RETIRED_RENDERER_SOURCE));
    let commit = git(&[
        "rev-parse",
        &format!("{RENDERER_CANONICAL_SOURCE}^{{commit}}"),
    ]);
    let tree = git(&[
        "rev-parse",
        &format!("{RENDERER_CANONICAL_SOURCE}^{{tree}}"),
    ]);
    assert_eq!(commit, RENDERER_CANONICAL_SOURCE);
    assert_eq!(tree, RENDERER_CANONICAL_TREE);
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
fn verifier_rejects_failed_real_owner_cleanup_claim() {
    assert_mutation_rejected(
        "const trace=record.focusRestorationTraces.find(trace => trace.interaction === 'focus-owner removal cleanup');trace.oldOwnerLive=true;trace.restored=false;",
        "owner-cleanup",
    );
}

#[test]
fn verifier_rejects_overlay_route_exclusivity_overclaim() {
    assert_mutation_rejected(
        "record.logs.stateTransitions.find(log => log.id === 'overlay-route-exclusivity').exclusive = false;",
        "overlay-route-exclusive",
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

#[test]
fn verifier_rejects_provisional_packet_claiming_final_or_passed() {
    assert_mutation_rejected("record.status = 'final';", "provisional-final");
    assert_mutation_rejected(
        "record.runtime.journeys.find(journey => journey.id === 'graph-connection-edit').status = 'passed';",
        "provisional-journey-passed",
    );
    assert_mutation_rejected(
        "record.gates.find(gate => gate.id === 'renderer-and-scale-quality').status = 'passed'; record.rendererEvidence.currentGraphLayoutStatus = 'passed';",
        "provisional-renderer-passed",
    );
}

#[test]
fn verifier_rejects_missing_gaps_or_unexpected_provisional_drift() {
    assert_mutation_rejected("record.knownGaps = [];", "provisional-gaps-omitted");
    assert_mutation_rejected(
        "record.source.provisionalGraphSourceDrift.push('README.md');",
        "unexpected-graph-drift",
    );
    assert_mutation_rejected(
        "record.source.provisionalModelColorContractDrift.push('README.md');",
        "unexpected-model-color-drift",
    );
    assert_mutation_rejected(
        "record.source.provisionalTimelineSourceDrift.push('README.md');",
        "unexpected-timeline-drift",
    );
    assert_mutation_rejected(
        "record.source.provisionalOverlayRecoveryContractDrift.push('README.md');",
        "unexpected-overlay-recovery-drift",
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

fn renderer_manifest() -> PathBuf {
    repo_root().join("evidence/stern-demo-vello-845/manifest.json")
}

fn git(args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("run git provenance check");
    assert!(output.status.success(), "git provenance command failed");
    String::from_utf8(output.stdout)
        .expect("git output is UTF-8")
        .trim()
        .to_owned()
}

fn assert_provisional(path: &Path) {
    let script = concat!(
        "const fs=require('fs');const r=JSON.parse(fs.readFileSync(process.argv[1],'utf8'));",
        "const passed=x=>x.filter(v=>v.status==='passed').length;",
        "const gate=id=>r.gates.find(v=>v.id===id).status;",
        "if(r.status!=='incomplete'||r.runtime.components.length!==34||",
        "passed(r.runtime.components)!==34||r.runtime.journeys.length!==7||",
        "passed(r.runtime.journeys)!==6||r.semanticSnapshots.length!==2||",
        "!r.publicConsumerAudit.passed||gate('renderer-and-scale-quality')!=='pending'||",
        "gate('deterministic-user-journeys')!=='pending'||",
        "gate('platform-integration')!=='passed'||r.knownGaps.length!==2||",
        "r.rendererEvidence.provenance!=='prior-baseline'||",
        "r.rendererEvidence.currentGraphLayoutStatus!=='pending'||",
        "r.source.provisionalGraphSourceDrift.length!==4||",
        "r.source.provisionalGraphContractDrift.length!==2||",
        "r.source.provisionalModelColorSourceDrift.length!==3||",
        "r.source.provisionalModelColorContractDrift.length!==2||",
        "r.source.provisionalTimelineSourceDrift.length!==4||",
        "r.source.provisionalTimelineContractDrift.length!==1||",
        "r.source.provisionalOverlayRecoverySourceDrift.length!==4||",
        "r.source.provisionalOverlayRecoveryContractDrift.length!==1)process.exit(1);",
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
