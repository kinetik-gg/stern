//! Process-level compatibility tests for the demo CLI's argument parser.
//!
//! These run the compiled `stern-demo` binary headlessly (no window, no GPU)
//! and assert on its exit code, stdout, and stderr.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_stern-demo"))
}

fn run(arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .output()
        .expect("run stern-demo binary")
}

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn fresh_directory(label: &str) -> PathBuf {
    let unique = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "stern-demo-cli-contract-{label}-{}-{unique}",
        std::process::id()
    ));
    if path.exists() {
        fs::remove_dir_all(&path).expect("clear stale fixture directory");
    }
    path
}

#[test]
fn unknown_argument_is_rejected_with_supported_flags_and_nonzero_exit() {
    let output = run(&["--bogus-flag"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("unrecognized argument `--bogus-flag`"),
        "{stderr}"
    );
    assert!(stderr.contains("--dump-identity-evidence"), "{stderr}");
}

#[test]
fn retired_dump_review_artifacts_flag_is_rejected() {
    let output = run(&[
        "--dump-review-artifacts",
        "s14-s10-s13-matrix",
        "--width",
        "1440",
        "--height",
        "900",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("unrecognized argument `--dump-review-artifacts`"),
        "{stderr}"
    );
}

#[test]
fn no_arguments_runs_the_default_frame_summary() {
    let output = run(&[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("Stern Integration Demo"), "{stdout}");
    assert!(stdout.contains("primitives"), "{stdout}");
    assert!(stdout.contains("semantic nodes"), "{stdout}");
}

#[test]
fn dump_identity_evidence_flag_still_works() {
    let directory = fresh_directory("identity-evidence");
    let output = run(&[
        "--dump-identity-evidence",
        directory.to_str().expect("utf8 path"),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let evidence = fs::read_to_string(directory.join("identity.txt")).expect("identity.txt");
    assert!(
        evidence.contains("title=Stern Integration Demo"),
        "{evidence}"
    );
    assert!(evidence.contains("package=stern-demo"), "{evidence}");
    assert!(evidence.contains("facade=stern"), "{evidence}");
    fs::remove_dir_all(&directory).expect("cleanup fixture directory");
}

#[test]
fn dump_identity_evidence_missing_value_keeps_the_prior_error_not_unknown_argument() {
    let output = run(&["--dump-identity-evidence"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(!stderr.contains("unrecognized argument"), "{stderr}");
    assert!(stderr.contains("missing evidence directory"), "{stderr}");
}
