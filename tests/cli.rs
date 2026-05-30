use std::process::Command;

fn tellci() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tellci"))
}

fn tellci_path() -> &'static str {
    env!("CARGO_BIN_EXE_tellci")
}

#[test]
fn fail_collects_and_finish_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let report = dir.path().join("tellci.xml");

    let fail_status = tellci()
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "fail",
            "Expected LICENSE file to exist",
        ])
        .status()
        .expect("run tellci fail");

    assert!(fail_status.success());

    let finish_status = tellci()
        .env_remove("GITHUB_ACTIONS")
        .args(["--file", report.to_str().expect("utf-8 path"), "finish"])
        .status()
        .expect("run tellci finish");

    assert_eq!(finish_status.code(), Some(1));
}

#[test]
fn fatal_fail_exits_with_failure_immediately() {
    let dir = tempfile::tempdir().expect("tempdir");
    let report = dir.path().join("tellci.xml");

    let status = tellci()
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "fail",
            "Blocker",
            "--fatal",
        ])
        .status()
        .expect("run tellci fail --fatal");

    assert_eq!(status.code(), Some(1));
    assert!(report.exists());
}

#[test]
fn all_passed_report_finishes_successfully() {
    let dir = tempfile::tempdir().expect("tempdir");
    let report = dir.path().join("tellci.xml");

    let pass_status = tellci()
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "pass",
            "README.md exists",
        ])
        .status()
        .expect("run tellci pass");

    assert!(pass_status.success());

    let finish_status = tellci()
        .env_remove("GITHUB_ACTIONS")
        .args(["--file", report.to_str().expect("utf-8 path"), "finish"])
        .status()
        .expect("run tellci finish");

    assert!(finish_status.success());
}

#[test]
fn platform_github_writes_summary_and_annotations() {
    let dir = tempfile::tempdir().expect("tempdir");
    let report = dir.path().join("tellci.xml");
    let summary = dir.path().join("summary.md");

    let fail_status = tellci()
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "fail",
            "Expected README.md to contain ## Installation",
        ])
        .status()
        .expect("run tellci fail");

    assert!(fail_status.success());

    let output = tellci()
        .env("GITHUB_STEP_SUMMARY", &summary)
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "finish",
            "--platform",
            "ghub",
        ])
        .output()
        .expect("run tellci finish --platform ghub");

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("::error title="));
    assert!(
        std::fs::read_to_string(summary)
            .expect("summary")
            .contains("### tellci")
    );
}

#[test]
fn finish_auto_detects_github_actions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let report = dir.path().join("tellci.xml");
    let summary = dir.path().join("summary.md");

    let fail_status = tellci()
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "fail",
            "Expected README.md to contain ## Installation",
        ])
        .status()
        .expect("run tellci fail");

    assert!(fail_status.success());

    let output = tellci()
        .env("GITHUB_ACTIONS", "true")
        .env_remove("GITLAB_CI")
        .env_remove("CI_SERVER_NAME")
        .env("GITHUB_STEP_SUMMARY", &summary)
        .args(["--file", report.to_str().expect("utf-8 path"), "finish"])
        .output()
        .expect("run tellci finish");

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("::error title="));
    assert!(
        std::fs::read_to_string(summary)
            .expect("summary")
            .contains("### tellci")
    );
}

#[test]
fn platform_gitlab_keeps_junit_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    let report = dir.path().join("tellci.xml");
    let summary = dir.path().join("summary.md");

    let fail_status = tellci()
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "fail",
            "Expected README.md to contain ## Installation",
        ])
        .status()
        .expect("run tellci fail");

    assert!(fail_status.success());

    let output = tellci()
        .env_remove("GITHUB_ACTIONS")
        .env("GITLAB_CI", "true")
        .env("GITHUB_STEP_SUMMARY", &summary)
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "finish",
            "--platform",
            "glab",
        ])
        .output()
        .expect("run tellci finish --platform glab");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!summary.exists());
}

#[test]
fn skip_records_skipped_testcase() {
    let dir = tempfile::tempdir().expect("tempdir");
    let report = dir.path().join("tellci.xml");

    let status = tellci()
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "skip",
            "PHPStan skipped because vendor/ is missing",
        ])
        .status()
        .expect("run tellci skip");

    assert!(status.success());
    let xml = std::fs::read_to_string(report).expect("report");
    assert!(xml.contains("skipped=\"1\""));
    assert!(xml.contains("<skipped message=\"PHPStan skipped because vendor/ is missing\"/>"));
}

#[test]
fn error_records_error_and_finish_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let report = dir.path().join("tellci.xml");

    let status = tellci()
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "error",
            "Tool crashed",
            "--details",
            "Process could not be started",
        ])
        .status()
        .expect("run tellci error");

    assert!(status.success());

    let finish_status = tellci()
        .env_remove("GITHUB_ACTIONS")
        .args(["--file", report.to_str().expect("utf-8 path"), "finish"])
        .status()
        .expect("run tellci finish");

    assert_eq!(finish_status.code(), Some(1));
    let xml = std::fs::read_to_string(report).expect("report");
    assert!(xml.contains("errors=\"1\""));
    assert!(xml.contains("<error message=\"Tool crashed\">Process could not be started</error>"));
}

#[test]
fn run_records_pass_for_successful_command() {
    let dir = tempfile::tempdir().expect("tempdir");
    let report = dir.path().join("tellci.xml");

    let status = tellci()
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "run",
            "tellci version works",
            "--",
            tellci_path(),
            "--version",
        ])
        .status()
        .expect("run tellci run");

    assert!(status.success());
    let xml = std::fs::read_to_string(report).expect("report");
    assert!(xml.contains("tests=\"1\""));
    assert!(xml.contains("failures=\"0\""));
    assert!(xml.contains("name=\"tellci version works\""));
}

#[test]
fn run_records_failure_for_nonzero_command_without_failing_immediately() {
    let dir = tempfile::tempdir().expect("tempdir");
    let report = dir.path().join("tellci.xml");

    let status = tellci()
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "run",
            "invalid tellci command fails",
            "--",
            tellci_path(),
            "not-a-command",
        ])
        .status()
        .expect("run tellci run");

    assert!(status.success());

    let finish_status = tellci()
        .env_remove("GITHUB_ACTIONS")
        .args(["--file", report.to_str().expect("utf-8 path"), "finish"])
        .status()
        .expect("run tellci finish");

    assert_eq!(finish_status.code(), Some(1));
    let xml = std::fs::read_to_string(report).expect("report");
    assert!(xml.contains("failures=\"1\""));
    assert!(xml.contains("Command:"));
    assert!(xml.contains("Exit code:"));
}

#[test]
fn platform_none_disables_auto_detected_output() {
    let dir = tempfile::tempdir().expect("tempdir");
    let report = dir.path().join("tellci.xml");
    let summary = dir.path().join("summary.md");

    let fail_status = tellci()
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "fail",
            "Expected README.md to contain ## Installation",
        ])
        .status()
        .expect("run tellci fail");

    assert!(fail_status.success());

    let output = tellci()
        .env("GITHUB_ACTIONS", "true")
        .env("GITHUB_STEP_SUMMARY", &summary)
        .args([
            "--file",
            report.to_str().expect("utf-8 path"),
            "finish",
            "--platform",
            "none",
        ])
        .output()
        .expect("run tellci finish --platform none");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!summary.exists());
}
