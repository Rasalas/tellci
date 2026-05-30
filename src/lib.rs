use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use quick_xml::{de::from_str, se::Serializer};
use serde::{Deserialize, Serialize};

pub const DEFAULT_FILE: &str = "tellci.xml";
pub const DEFAULT_SUITE: &str = "tellci";
pub const DEFAULT_CLASS: &str = "tellci";

#[derive(Debug, Clone)]
pub struct AddOptions {
    pub suite: Option<String>,
    pub class: String,
    pub details: Option<String>,
}

impl Default for AddOptions {
    fn default() -> Self {
        Self {
            suite: None,
            class: DEFAULT_CLASS.to_string(),
            details: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportStatus {
    pub tests: usize,
    pub failures: usize,
    pub errors: usize,
    pub skipped: usize,
}

impl ReportStatus {
    pub fn is_success(self) -> bool {
        self.failures == 0 && self.errors == 0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "testsuite")]
struct TestSuite {
    #[serde(rename = "@name", default = "default_suite_name")]
    name: String,
    #[serde(rename = "@tests", default)]
    tests: usize,
    #[serde(rename = "@failures", default)]
    failures: usize,
    #[serde(rename = "@errors", default)]
    errors: usize,
    #[serde(rename = "@skipped", default)]
    skipped: usize,
    #[serde(rename = "testcase", default)]
    testcases: Vec<TestCase>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TestCase {
    #[serde(rename = "@classname", default = "default_class_name")]
    classname: String,
    #[serde(rename = "@name", default)]
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<ReportText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ReportText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped: Option<Skipped>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ReportText {
    #[serde(rename = "@message", default)]
    message: String,
    #[serde(rename = "$text", default)]
    body: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Skipped {
    #[serde(rename = "@message", default)]
    message: String,
}

fn default_suite_name() -> String {
    DEFAULT_SUITE.to_string()
}

fn default_class_name() -> String {
    DEFAULT_CLASS.to_string()
}

impl TestSuite {
    fn empty(name: impl Into<String>) -> Self {
        let mut suite = Self {
            name: name.into(),
            tests: 0,
            failures: 0,
            errors: 0,
            skipped: 0,
            testcases: Vec::new(),
        };
        suite.recalculate();
        suite
    }

    fn status(&self) -> ReportStatus {
        ReportStatus {
            tests: self.tests,
            failures: self.failures,
            errors: self.errors,
            skipped: self.skipped,
        }
    }

    fn recalculate(&mut self) {
        self.tests = self.testcases.len();
        self.failures = self
            .testcases
            .iter()
            .filter(|testcase| testcase.failure.is_some())
            .count();
        self.errors = self
            .testcases
            .iter()
            .filter(|testcase| testcase.error.is_some())
            .count();
        self.skipped = self
            .testcases
            .iter()
            .filter(|testcase| testcase.skipped.is_some())
            .count();
    }

    fn passed(&self) -> usize {
        self.tests
            .saturating_sub(self.failures)
            .saturating_sub(self.errors)
            .saturating_sub(self.skipped)
    }
}

pub fn pass(path: &Path, message: impl Into<String>, options: AddOptions) -> Result<ReportStatus> {
    let mut suite = load_or_empty(path, options.suite.as_deref())?;
    if let Some(suite_name) = options.suite {
        suite.name = suite_name;
    }
    suite.testcases.push(TestCase {
        classname: options.class,
        name: message.into(),
        failure: None,
        error: None,
        skipped: None,
    });
    save(path, suite)
}

pub fn fail(path: &Path, message: impl Into<String>, options: AddOptions) -> Result<ReportStatus> {
    let mut suite = load_or_empty(path, options.suite.as_deref())?;
    if let Some(suite_name) = options.suite {
        suite.name = suite_name;
    }

    let message = message.into();
    let body = options.details.unwrap_or_else(|| message.clone());

    suite.testcases.push(TestCase {
        classname: options.class,
        name: message.clone(),
        failure: Some(ReportText { message, body }),
        error: None,
        skipped: None,
    });
    save(path, suite)
}

pub fn finish(path: &Path) -> Result<ReportStatus> {
    let suite = load_or_empty(path, None)?;
    save(path, suite)
}

pub fn github_markdown(path: &Path) -> Result<String> {
    let mut suite = load_or_empty(path, None)?;
    suite.recalculate();
    Ok(render_github_markdown(&suite))
}

pub fn github_annotations(path: &Path) -> Result<String> {
    let mut suite = load_or_empty(path, None)?;
    suite.recalculate();
    Ok(render_github_annotations(&suite))
}

pub fn reset(path: &Path, suite_name: Option<&str>) -> Result<ReportStatus> {
    save(
        path,
        TestSuite::empty(suite_name.unwrap_or(DEFAULT_SUITE).to_string()),
    )
}

pub fn status(path: &Path) -> Result<ReportStatus> {
    let mut suite = load_or_empty(path, None)?;
    suite.recalculate();
    Ok(suite.status())
}

pub fn default_path() -> PathBuf {
    PathBuf::from(DEFAULT_FILE)
}

fn load_or_empty(path: &Path, suite_name: Option<&str>) -> Result<TestSuite> {
    if !path.exists() {
        return Ok(TestSuite::empty(suite_name.unwrap_or(DEFAULT_SUITE)));
    }

    let xml = fs::read_to_string(path)
        .with_context(|| format!("failed to read report file {}", path.display()))?;
    if xml.trim().is_empty() {
        return Ok(TestSuite::empty(suite_name.unwrap_or(DEFAULT_SUITE)));
    }

    let mut suite: TestSuite = from_str(&xml)
        .with_context(|| format!("failed to parse JUnit XML report {}", path.display()))?;
    suite.recalculate();
    Ok(suite)
}

fn save(path: &Path, mut suite: TestSuite) -> Result<ReportStatus> {
    suite.recalculate();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create report directory {}", parent.display()))?;
    }

    let status = suite.status();
    let mut xml = String::new();
    let mut serializer = Serializer::new(&mut xml);
    serializer.indent(' ', 2);
    suite
        .serialize(serializer)
        .context("failed to serialize JUnit XML report")?;
    xml.push('\n');
    fs::write(path, xml)
        .with_context(|| format!("failed to write report file {}", path.display()))?;
    Ok(status)
}

fn render_github_markdown(suite: &TestSuite) -> String {
    let mut markdown = String::new();
    markdown.push_str("### tellci\n\n");
    markdown.push_str("| Result | Count |\n");
    markdown.push_str("| --- | ---: |\n");
    markdown.push_str(&format!("| Tests | {} |\n", suite.tests));
    markdown.push_str(&format!("| Passed | {} |\n", suite.passed()));
    markdown.push_str(&format!("| Failures | {} |\n", suite.failures));
    markdown.push_str(&format!("| Errors | {} |\n", suite.errors));
    markdown.push_str(&format!("| Skipped | {} |\n", suite.skipped));

    let failures = suite
        .testcases
        .iter()
        .filter_map(|testcase| testcase.failure.as_ref().map(|failure| (testcase, failure)))
        .collect::<Vec<_>>();

    if !failures.is_empty() {
        markdown.push_str("\n#### Failures\n\n");
        for (testcase, failure) in failures {
            markdown.push_str(&format!("- **{}**", markdown_text(&testcase.name)));
            if !failure.body.is_empty() && failure.body != failure.message {
                markdown.push_str(&format!(": {}", markdown_text(&failure.body)));
            }
            markdown.push('\n');
        }
    }

    markdown
}

fn render_github_annotations(suite: &TestSuite) -> String {
    let mut annotations = String::new();

    for testcase in &suite.testcases {
        if let Some(failure) = &testcase.failure {
            annotations.push_str(&format!(
                "::error title={}::{}\n",
                workflow_command_property(&testcase.name),
                workflow_command_data(&failure.body)
            ));
        }

        if let Some(error) = &testcase.error {
            annotations.push_str(&format!(
                "::error title={}::{}\n",
                workflow_command_property(&testcase.name),
                workflow_command_data(&error.body)
            ));
        }
    }

    annotations
}

fn markdown_text(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', "<br>")
}

fn workflow_command_property(text: &str) -> String {
    workflow_command_data(text)
        .replace(':', "%3A")
        .replace(',', "%2C")
}

fn workflow_command_data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report_path() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("tellci.xml");
        (dir, path)
    }

    #[test]
    fn pass_creates_report_with_successful_testcase() {
        let (_dir, path) = report_path();

        let status = pass(&path, "Composer install works", AddOptions::default()).unwrap();

        assert_eq!(status.tests, 1);
        assert_eq!(status.failures, 0);

        let xml = fs::read_to_string(path).unwrap();
        assert!(xml.contains("tests=\"1\""));
        assert!(xml.contains("failures=\"0\""));
        assert!(xml.contains("name=\"Composer install works\""));
        assert!(!xml.contains("<failure"));
    }

    #[test]
    fn fail_appends_failure_without_failing_immediately() {
        let (_dir, path) = report_path();

        pass(&path, "README.md exists", AddOptions::default()).unwrap();
        let status = fail(
            &path,
            "Expected LICENSE file to exist",
            AddOptions::default(),
        )
        .unwrap();

        assert_eq!(status.tests, 2);
        assert_eq!(status.failures, 1);

        let finished = finish(&path).unwrap();
        assert_eq!(finished.failures, 1);
        assert!(!finished.is_success());
    }

    #[test]
    fn failure_details_are_used_as_body() {
        let (_dir, path) = report_path();

        fail(
            &path,
            "Expected README.md to contain ## Installation",
            AddOptions {
                details: Some("README.md was present but the heading was missing".to_string()),
                ..AddOptions::default()
            },
        )
        .unwrap();

        let xml = fs::read_to_string(path).unwrap();
        assert!(xml.contains("message=\"Expected README.md to contain ## Installation\""));
        assert!(xml.contains("README.md was present but the heading was missing"));
    }

    #[test]
    fn xml_content_is_escaped() {
        let (_dir, path) = report_path();

        fail(
            &path,
            "Expected A < B & C > D",
            AddOptions {
                details: Some("Actual: <tag attr=\"value\">".to_string()),
                ..AddOptions::default()
            },
        )
        .unwrap();

        let xml = fs::read_to_string(&path).unwrap();
        assert!(xml.contains("Expected A &lt; B &amp; C &gt; D"));
        assert!(xml.contains("&lt;tag attr=\"value\"&gt;"));
        assert_eq!(finish(&path).unwrap().failures, 1);
    }

    #[test]
    fn reset_overwrites_existing_report() {
        let (_dir, path) = report_path();

        fail(
            &path,
            "Expected LICENSE file to exist",
            AddOptions::default(),
        )
        .unwrap();
        let status = reset(&path, None).unwrap();

        assert_eq!(status.tests, 0);
        assert_eq!(finish(&path).unwrap().failures, 0);
    }

    #[test]
    fn custom_file_parent_is_created() {
        let (_dir, path) = report_path();
        let nested = path.parent().unwrap().join("reports/tellci.xml");

        pass(&nested, "Nested report works", AddOptions::default()).unwrap();

        assert!(nested.exists());
    }

    #[test]
    fn custom_suite_and_class_are_written() {
        let (_dir, path) = report_path();

        pass(
            &path,
            "Composer install works",
            AddOptions {
                suite: Some("PHP".to_string()),
                class: "Composer".to_string(),
                details: None,
            },
        )
        .unwrap();

        let xml = fs::read_to_string(path).unwrap();
        assert!(xml.contains("<testsuite name=\"PHP\""));
        assert!(xml.contains("classname=\"Composer\""));
    }

    #[test]
    fn github_markdown_summarizes_report() {
        let (_dir, path) = report_path();

        pass(&path, "README.md exists", AddOptions::default()).unwrap();
        fail(
            &path,
            "Expected LICENSE | file to exist",
            AddOptions {
                details: Some("No LICENSE file was found\nAdd one.".to_string()),
                ..AddOptions::default()
            },
        )
        .unwrap();

        let markdown = github_markdown(&path).unwrap();

        assert!(markdown.contains("| Tests | 2 |"));
        assert!(markdown.contains("| Passed | 1 |"));
        assert!(markdown.contains("| Failures | 1 |"));
        assert!(markdown.contains("Expected LICENSE \\| file to exist"));
        assert!(markdown.contains("No LICENSE file was found<br>Add one."));
    }

    #[test]
    fn github_annotations_escape_workflow_command_data() {
        let (_dir, path) = report_path();

        fail(
            &path,
            "Expected foo:bar,baz",
            AddOptions {
                details: Some("Actual: 50%\nMissing value".to_string()),
                ..AddOptions::default()
            },
        )
        .unwrap();

        let annotations = github_annotations(&path).unwrap();

        assert_eq!(
            annotations,
            "::error title=Expected foo%3Abar%2Cbaz::Actual: 50%25%0AMissing value\n"
        );
    }
}
