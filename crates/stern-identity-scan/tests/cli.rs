//! Process-level compatibility tests for the identity scanner CLI.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use support::TempDir;

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_stern-identity-scan"))
}

fn workspace_root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn configured_repository() -> String {
    let owner = [107_u8, 105, 110, 101, 116, 105, 107, 45, 103, 103]
        .into_iter()
        .map(char::from)
        .collect::<String>();
    format!("{owner}/stern")
}

fn marker_base() -> String {
    [107_u8, 105, 110, 101, 116, 105, 107]
        .into_iter()
        .map(char::from)
        .collect()
}

fn run(cwd: &Path, arguments: &[String]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(cwd)
        .output()
        .unwrap()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn filesystem_arguments(root: &Path, path: &str) -> Vec<String> {
    vec![
        "--root".to_owned(),
        root.to_string_lossy().into_owned(),
        "--scope".to_owned(),
        "filesystem".to_owned(),
        "--configured-repository".to_owned(),
        configured_repository(),
        "--path".to_owned(),
        path.to_owned(),
    ]
}

fn assert_failure_path(output: &Output, kind: &str, path: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let failure: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["status"], "fail");
    assert_eq!(failure["match"]["kind"], kind);
    assert_eq!(failure["match"]["path"], path);
    assert_eq!(failure["match"]["normalizedLength"], 7);
    assert_eq!(
        failure["match"]["markerHash"],
        "d1f8b80df0f04f9fb9d2301f8811562cc16010891492393f88ec8c3a4f2890a5"
    );
}

#[test]
fn self_test_stdout_stderr_and_exit_match_the_original() {
    let root = workspace_root();
    let output = run(&root, &strings(&["--root", ".", "--self-test"]));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "{\"status\":\"pass\",\"suite\":\"identity-scan\",\"tests\":11}\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn metadata_success_and_violation_match_original_process_contract() {
    let root = workspace_root();
    let configured = configured_repository();
    let success = run(
        &root,
        &[
            "--scope".to_owned(),
            "metadata".to_owned(),
            "--configured-repository".to_owned(),
            configured.clone(),
            "--metadata".to_owned(),
            format!("repository={configured}"),
            "--metadata".to_owned(),
            format!("origin=git@github.com:{configured}.git"),
        ],
    );
    assert_eq!(success.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(success.stdout).unwrap(),
        "{\"status\":\"pass\",\"scope\":\"metadata\",\"textFiles\":0}\n"
    );
    assert!(success.stderr.is_empty());

    let owner = configured.split('/').next().unwrap().to_owned();
    let failure = run(
        &root,
        &[
            "--scope".to_owned(),
            "metadata".to_owned(),
            "--configured-repository".to_owned(),
            configured,
            "--metadata".to_owned(),
            format!("owner={owner}"),
        ],
    );
    assert_eq!(failure.status.code(), Some(1));
    assert!(failure.stdout.is_empty());
    assert_eq!(
        String::from_utf8(failure.stderr).unwrap(),
        "{\"status\":\"fail\",\"match\":{\"kind\":\"metadata\",\"keySha256\":\"4c1029697ee358715d3a14a2add817c4b01651440de808371f78165ac90dc581\",\"normalizedLength\":7,\"markerHash\":\"d1f8b80df0f04f9fb9d2301f8811562cc16010891492393f88ec8c3a4f2890a5\"}}\n"
    );
}

#[test]
fn invalid_arguments_use_exact_error_stream_and_exit_two() {
    let root = workspace_root();
    for (arguments, message) in [
        (strings(&["--bogus"]), "unknown argument"),
        (strings(&["--root"]), "missing argument value"),
        (
            strings(&["--scope", "other", "--configured-repository", "owner/stern"]),
            "scope must be tracked, filesystem, or metadata",
        ),
        (
            strings(&[
                "--scope",
                "filesystem",
                "--configured-repository",
                "owner/stern",
            ]),
            "filesystem scope requires path",
        ),
        (
            strings(&[
                "--scope",
                "metadata",
                "--configured-repository",
                "owner/other",
            ]),
            "configured repository must name the canonical repository",
        ),
        (
            strings(&[
                "--scope",
                "metadata",
                "--configured-repository",
                "owner/stern",
                "--metadata",
                "malformed",
            ]),
            "metadata must use key=value",
        ),
    ] {
        let output = run(&root, &arguments);
        assert_eq!(output.status.code(), Some(2), "arguments: {arguments:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            format!("{{\"status\":\"error\",\"message\":\"{message}\"}}\n")
        );
    }
}

#[test]
fn filesystem_scope_reports_success_failure_and_root_escape() {
    let fixture = TempDir::new("filesystem");
    fs::write(fixture.path().join("clean.txt"), "Stern — UTF-8").unwrap();
    fs::write(
        fixture.path().join("violation.txt"),
        format!("prefix-{}-suffix", marker_base()),
    )
    .unwrap();
    let configured = configured_repository();
    let base = vec![
        "--root".to_owned(),
        fixture.path().to_string_lossy().into_owned(),
        "--scope".to_owned(),
        "filesystem".to_owned(),
        "--configured-repository".to_owned(),
        configured,
        "--path".to_owned(),
    ];

    let mut success_args = base.clone();
    success_args.push("clean.txt".to_owned());
    let success = run(&workspace_root(), &success_args);
    assert_eq!(success.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(success.stdout).unwrap(),
        "{\"status\":\"pass\",\"scope\":\"filesystem\",\"textFiles\":1}\n"
    );
    assert!(success.stderr.is_empty());

    let mut failure_args = base.clone();
    failure_args.push("violation.txt".to_owned());
    let failure = run(&workspace_root(), &failure_args);
    assert_eq!(failure.status.code(), Some(1));
    assert!(failure.stdout.is_empty());
    let failure_json: serde_json::Value = serde_json::from_slice(&failure.stderr).unwrap();
    assert_eq!(failure_json["status"], "fail");
    assert_eq!(failure_json["match"]["kind"], "filesystem-content");
    assert_eq!(failure_json["match"]["path"], "violation.txt");

    let mut escape_args = base;
    escape_args.push("../outside".to_owned());
    let escape = run(&workspace_root(), &escape_args);
    assert_eq!(escape.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(escape.stderr).unwrap(),
        "{\"status\":\"error\",\"message\":\"scan path escaped root\"}\n"
    );
}

#[test]
fn tracked_scope_honors_git_ignore_and_counts_effective_files() {
    let fixture = TempDir::new("tracked");
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(fixture.path())
        .status()
        .unwrap();
    assert!(status.success());
    fs::write(fixture.path().join(".gitignore"), "ignored.txt\n").unwrap();
    fs::write(fixture.path().join("clean.txt"), "clean").unwrap();
    fs::write(fixture.path().join("ignored.txt"), marker_base()).unwrap();
    let output = run(
        &workspace_root(),
        &[
            "--root".to_owned(),
            fixture.path().to_string_lossy().into_owned(),
            "--scope".to_owned(),
            "tracked".to_owned(),
            "--configured-repository".to_owned(),
            configured_repository(),
        ],
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "{\"status\":\"pass\",\"scope\":\"tracked\",\"textFiles\":2}\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn rooted_filesystem_paths_match_original_join_behavior() {
    let fixture = TempDir::new("rooted-process");
    fs::write(fixture.path().join("clean.txt"), "clean").unwrap();
    #[cfg(windows)]
    let rooted_paths = ["/clean.txt", "\\clean.txt"];
    #[cfg(not(windows))]
    let rooted_paths = ["/clean.txt"];
    for rooted in rooted_paths {
        let output = run(
            &workspace_root(),
            &filesystem_arguments(fixture.path(), rooted),
        );
        assert_eq!(output.status.code(), Some(0), "path {rooted:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "{\"status\":\"pass\",\"scope\":\"filesystem\",\"textFiles\":1}\n"
        );
        assert!(output.stderr.is_empty());
    }

    if cfg!(windows) {
        for (rooted, joined) in [
            ("C:/missing.txt", "C:\\missing.txt"),
            ("C:\\missing.txt", "C:\\missing.txt"),
            ("//server/share/missing.txt", "server\\share\\missing.txt"),
            (
                "\\\\server\\share\\missing.txt",
                "server\\share\\missing.txt",
            ),
        ] {
            let output = run(
                &workspace_root(),
                &filesystem_arguments(fixture.path(), rooted),
            );
            assert_eq!(output.status.code(), Some(2), "path {rooted:?}");
            assert!(output.stdout.is_empty());
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["status"], "error");
            assert_eq!(
                error["message"],
                format!(
                    "ENOENT: no such file or directory, lstat '{}\\{joined}'",
                    fixture.path().display()
                )
            );
        }
    }
}

#[test]
fn ecmascript_url_boundaries_match_original_process_behavior() {
    let fixture = TempDir::new("url-boundary-process");
    let configured = configured_repository();
    let canonical = [
        format!("https://github.com/{configured}"),
        format!("git@github.com:{configured}.git"),
        format!("ssh://git@github.com/{configured}.git"),
    ];
    fs::write(
        fixture.path().join("canonical.txt"),
        canonical
            .iter()
            .map(|url| format!("{url}\u{FEFF}tail"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    let success = run(
        &workspace_root(),
        &filesystem_arguments(fixture.path(), "canonical.txt"),
    );
    assert_eq!(success.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(success.stdout).unwrap(),
        "{\"status\":\"pass\",\"scope\":\"filesystem\",\"textFiles\":1}\n"
    );
    assert!(success.stderr.is_empty());

    fs::write(
        fixture.path().join("noncanonical.txt"),
        format!("{}\u{200B}", canonical[0]),
    )
    .unwrap();
    assert_failure_path(
        &run(
            &workspace_root(),
            &filesystem_arguments(fixture.path(), "noncanonical.txt"),
        ),
        "filesystem-content",
        "noncanonical.txt",
    );
}

#[test]
fn filesystem_first_failure_uses_pinned_locale_compare_order() {
    let fixture = TempDir::new("filesystem-collation");
    for (directory, expected) in [
        ("punctuation", "punctuation/_x.txt"),
        ("accent", "accent/á.txt"),
        ("nested", "nested/a/fail.txt"),
    ] {
        let paths = match directory {
            "punctuation" => vec!["punctuation/-x.txt", "punctuation/_x.txt"],
            "accent" => vec!["accent/b.txt", "accent/ä.txt", "accent/á.txt"],
            "nested" => vec![
                "nested/a/fail.txt",
                "nested/a-z/fail.txt",
                "nested/a_z/fail.txt",
            ],
            _ => unreachable!(),
        };
        for path in paths {
            let path = fixture.path().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, marker_base()).unwrap();
        }
        assert_failure_path(
            &run(
                &workspace_root(),
                &filesystem_arguments(fixture.path(), directory),
            ),
            "filesystem-content",
            expected,
        );
    }
}

#[test]
fn tracked_first_failure_uses_pinned_locale_compare_order() {
    let fixture = TempDir::new("tracked-collation");
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(fixture.path())
        .status()
        .unwrap();
    assert!(status.success());
    fs::write(fixture.path().join("-x.txt"), marker_base()).unwrap();
    fs::write(fixture.path().join("_x.txt"), marker_base()).unwrap();
    let output = run(
        &workspace_root(),
        &[
            "--root".to_owned(),
            fixture.path().to_string_lossy().into_owned(),
            "--scope".to_owned(),
            "tracked".to_owned(),
            "--configured-repository".to_owned(),
            configured_repository(),
        ],
    );
    assert_failure_path(&output, "tracked-content", "_x.txt");

    fs::remove_file(fixture.path().join("-x.txt")).unwrap();
    fs::remove_file(fixture.path().join("_x.txt")).unwrap();
    for path in ["a/fail.txt", "a-z/fail.txt", "a_z/fail.txt"] {
        let path = fixture.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, marker_base()).unwrap();
    }
    let nested = run(
        &workspace_root(),
        &[
            "--root".to_owned(),
            fixture.path().to_string_lossy().into_owned(),
            "--scope".to_owned(),
            "tracked".to_owned(),
            "--configured-repository".to_owned(),
            configured_repository(),
        ],
    );
    assert_failure_path(&nested, "tracked-content", "a_z/fail.txt");
}
