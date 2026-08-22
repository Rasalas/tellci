use std::process::{Command, Output};

fn tellci() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tellci"))
}

fn tellci_path() -> &'static str {
    env!("CARGO_BIN_EXE_tellci")
}

fn temp_path(name: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    (dir, path.to_str().expect("utf-8 path").to_string())
}

fn report() -> (tempfile::TempDir, String) {
    temp_path("tellci.xml")
}

fn summary() -> (tempfile::TempDir, String) {
    temp_path("summary.md")
}

fn run(args: &[&str]) -> Output {
    tellci().args(args).output().expect("run tellci")
}

fn run_with(envs: &[(&str, &str)], removals: &[&str], args: &[&str]) -> Output {
    let mut command = tellci();
    for (key, value) in envs {
        command.env(key, value);
    }
    for key in removals {
        command.env_remove(key);
    }
    command.args(args).output().expect("run tellci")
}

#[test]
fn fail_collects_and_finish_fails() {
    let (_dir, report) = report();

    assert!(
        run(&["--file", &report, "fail", "Expected LICENSE file to exist"])
            .status
            .success()
    );

    let finish = run_with(&[], &["GITHUB_ACTIONS"], &["--file", &report, "finish"]);

    assert_eq!(finish.status.code(), Some(1));
}

#[test]
fn tellci_file_env_selects_report_path() {
    let (_dir, report) = report();

    let fail = run_with(
        &[("TELLCI_FILE", report.as_str())],
        &[],
        &["pass", "Env var report works"],
    );
    assert!(fail.status.success());
    assert!(
        std::fs::read_to_string(&report)
            .unwrap()
            .contains("tests=\"1\"")
    );

    let status = run_with(&[("TELLCI_FILE", report.as_str())], &[], &["status"]);
    assert!(status.status.success());
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(stdout.contains("1 tests"));
    assert!(stdout.contains("0 failures"));
}

#[test]
fn fatal_fail_exits_with_failure_immediately() {
    let (_dir, report) = report();

    let status = run(&["--file", &report, "fail", "Blocker", "--fatal"]);

    assert_eq!(status.status.code(), Some(1));
    assert!(std::path::Path::new(&report).exists());
}

#[test]
fn all_passed_report_finishes_successfully() {
    let (_dir, report) = report();

    assert!(
        run(&["--file", &report, "pass", "README.md exists"])
            .status
            .success()
    );

    let finish = run_with(&[], &["GITHUB_ACTIONS"], &["--file", &report, "finish"]);

    assert!(finish.status.success());
}

#[test]
fn platform_github_writes_summary_and_annotations() {
    let (_summary_dir, summary) = summary();
    let (_report_dir, report) = report();

    assert!(
        run(&[
            "--file",
            &report,
            "fail",
            "Expected README.md to contain ## Installation"
        ])
        .status
        .success()
    );

    let output = run_with(
        &[("GITHUB_STEP_SUMMARY", summary.as_str())],
        &[],
        &["--file", &report, "finish", "--platform", "ghub"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("::error title="));
    assert!(
        std::fs::read_to_string(&summary)
            .expect("summary")
            .contains("### tellci")
    );
}

#[test]
fn finish_auto_detects_github_actions() {
    let (_summary_dir, summary) = summary();
    let (_report_dir, report) = report();

    assert!(
        run(&[
            "--file",
            &report,
            "fail",
            "Expected README.md to contain ## Installation"
        ])
        .status
        .success()
    );

    let output = run_with(
        &[
            ("GITHUB_ACTIONS", "true"),
            ("GITHUB_STEP_SUMMARY", summary.as_str()),
        ],
        &["GITLAB_CI", "CI_SERVER_NAME"],
        &["--file", &report, "finish"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("::error title="));
    assert!(
        std::fs::read_to_string(&summary)
            .expect("summary")
            .contains("### tellci")
    );
}

#[test]
fn platform_gitlab_keeps_junit_only() {
    let (_summary_dir, summary) = summary();
    let (_report_dir, report) = report();

    assert!(
        run(&[
            "--file",
            &report,
            "fail",
            "Expected README.md to contain ## Installation"
        ])
        .status
        .success()
    );

    let output = run_with(
        &[
            ("GITLAB_CI", "true"),
            ("GITHUB_STEP_SUMMARY", summary.as_str()),
        ],
        &["GITHUB_ACTIONS"],
        &["--file", &report, "finish", "--platform", "glab"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!std::path::Path::new(&summary).exists());
}

#[test]
fn skip_records_skipped_testcase() {
    let (_dir, report) = report();

    let status = run(&[
        "--file",
        &report,
        "skip",
        "PHPStan skipped because vendor/ is missing",
    ]);

    assert!(status.status.success());
    let xml = std::fs::read_to_string(&report).expect("report");
    assert!(xml.contains("skipped=\"1\""));
    assert!(xml.contains("<skipped message=\"PHPStan skipped because vendor/ is missing\"/>"));
}

#[test]
fn error_records_error_and_finish_fails() {
    let (_dir, report) = report();

    let status = run(&[
        "--file",
        &report,
        "error",
        "Tool crashed",
        "--details",
        "Process could not be started",
    ]);

    assert!(status.status.success());

    let finish = run_with(&[], &["GITHUB_ACTIONS"], &["--file", &report, "finish"]);

    assert_eq!(finish.status.code(), Some(1));
    let xml = std::fs::read_to_string(&report).expect("report");
    assert!(xml.contains("errors=\"1\""));
    assert!(xml.contains("<error message=\"Tool crashed\">Process could not be started</error>"));
}

#[test]
fn run_records_pass_for_successful_command() {
    let (_dir, report) = report();

    let status = run(&[
        "--file",
        &report,
        "run",
        "tellci version works",
        "--",
        tellci_path(),
        "--version",
    ]);

    assert!(status.status.success());
    let xml = std::fs::read_to_string(&report).expect("report");
    assert!(xml.contains("tests=\"1\""));
    assert!(xml.contains("failures=\"0\""));
    assert!(xml.contains("name=\"tellci version works\""));
}

#[test]
fn run_records_failure_for_nonzero_command_without_failing_immediately() {
    let (_dir, report) = report();

    let status = run(&[
        "--file",
        &report,
        "run",
        "invalid tellci command fails",
        "--",
        tellci_path(),
        "not-a-command",
    ]);

    assert!(status.status.success());

    let finish = run_with(&[], &["GITHUB_ACTIONS"], &["--file", &report, "finish"]);

    assert_eq!(finish.status.code(), Some(1));
    let xml = std::fs::read_to_string(&report).expect("report");
    assert!(xml.contains("failures=\"1\""));
    assert!(xml.contains("Command:"));
    assert!(xml.contains("Exit code:"));
}

#[test]
fn run_missing_binary_records_error_testcase() {
    let (_dir, report) = report();

    let status = run(&[
        "--file",
        &report,
        "run",
        "Missing tool is recorded as error",
        "--",
        "/nonexistent/tellci-missing-binary",
    ]);

    assert!(status.status.success());

    let xml = std::fs::read_to_string(&report).expect("report");
    assert!(xml.contains("errors=\"1\""));
    assert!(xml.contains("<error message=\"Missing tool is recorded as error\""));
    assert!(xml.contains("Failed to start command"));
}

#[test]
fn reset_cli_clears_existing_report() {
    let (_dir, report) = report();

    assert!(
        run(&["--file", &report, "fail", "Old finding"])
            .status
            .success()
    );
    assert!(run(&["--file", &report, "reset"]).status.success());

    let status = run(&["--file", &report, "status"]);
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(stdout.contains("0 tests"));
    assert!(stdout.contains("0 failures"));

    let finish = run_with(&[], &["GITHUB_ACTIONS"], &["--file", &report, "finish"]);
    assert!(finish.status.success());
}

#[test]
fn corrupt_report_fails_with_io_error_code() {
    let (_dir, report) = report();
    std::fs::write(&report, "not xml at all").expect("write corrupt report");

    assert_eq!(
        run(&["--file", &report, "fail", "Message"]).status.code(),
        Some(2)
    );
    assert_eq!(run(&["--file", &report, "finish"]).status.code(), Some(2));
    assert_eq!(run(&["--file", &report, "status"]).status.code(), Some(2));
}

#[test]
fn empty_report_file_is_treated_as_empty_suite() {
    let (_dir, report) = report();
    std::fs::write(&report, "").expect("write empty report");

    let status = run(&["--file", &report, "status"]);
    assert!(status.status.success());
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(stdout.contains("0 tests"));
}

#[test]
fn platform_none_disables_auto_detected_output() {
    let (_summary_dir, summary) = summary();
    let (_report_dir, report) = report();

    assert!(
        run(&[
            "--file",
            &report,
            "fail",
            "Expected README.md to contain ## Installation"
        ])
        .status
        .success()
    );

    let output = run_with(
        &[
            ("GITHUB_ACTIONS", "true"),
            ("GITHUB_STEP_SUMMARY", summary.as_str()),
        ],
        &[],
        &["--file", &report, "finish", "--platform", "none"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!std::path::Path::new(&summary).exists());
}
