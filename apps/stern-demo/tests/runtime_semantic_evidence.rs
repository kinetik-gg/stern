//! Focused generator and stale-evidence rejection checks.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn generator_records_honest_current_runtime_packet() {
    let evidence = temp("generated");
    generate(&evidence);
    assert_provisional(&evidence);
    verify(&evidence, true);
    let _ = fs::remove_file(evidence);
}

#[test]
fn verifier_rejects_stale_source_and_premature_platform_gate_claims() {
    let evidence = temp("tampered");
    generate(&evidence);
    mutate(&evidence, "record.source.tree = '0'.repeat(40);");
    verify(&evidence, false);

    generate(&evidence);
    mutate(
        &evidence,
        "record.gates.find(gate => gate.id === 'platform-integration').status = 'passed';",
    );
    verify(&evidence, false);
    let _ = fs::remove_file(evidence);
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
    generate(&evidence);
    mutate(&evidence, mutation);
    verify(&evidence, false);
    let _ = fs::remove_file(evidence);
}

fn assert_provisional(path: &Path) {
    let script = concat!(
        "const fs=require('fs');const r=JSON.parse(fs.readFileSync(process.argv[1],'utf8'));",
        "const passed=x=>x.filter(v=>v.status==='passed').length;",
        "const gate=id=>r.gates.find(v=>v.id===id).status;",
        "if(r.status!=='incomplete'||r.runtime.components.length!==34||",
        "passed(r.runtime.components)!==34||r.runtime.journeys.length!==7||",
        "passed(r.runtime.journeys)!==7||r.semanticSnapshots.length!==2||",
        "!r.publicConsumerAudit.passed||gate('renderer-and-scale-quality')!=='passed'||",
        "gate('platform-integration')!=='pending')process.exit(1);",
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
