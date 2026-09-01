use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use tellci::{AddOptions, CiProvider, DEFAULT_CLASS, DEFAULT_FILE, DEFAULT_SUITE};

#[derive(Debug, Parser)]
#[command(version, about = "Write simple CI feedback as JUnit XML")]
struct Cli {
    #[arg(long, global = true, env = "TELLCI_FILE", default_value = DEFAULT_FILE)]
    file: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Append a successful testcase.
    Pass(AddCommand),

    /// Append a failed testcase. This exits 0 unless --fatal is used.
    Fail(IssueCommand),

    /// Append an errored testcase. This exits 0 unless --fatal is used.
    Error(IssueCommand),

    /// Append a skipped testcase.
    Skip(SkipCommand),

    /// Run a command and append pass or fail based on its exit status.
    Run(RunCommand),

    /// Exit 1 when the report contains failures or errors.
    Finish(FinishCommand),

    /// Replace the report with an empty testsuite.
    Reset {
        #[arg(long, default_value = DEFAULT_SUITE)]
        suite: String,
    },

    /// Print report counters.
    Status,
}

#[derive(Debug, Parser)]
struct AddCommand {
    message: String,

    #[arg(long = "class", default_value = DEFAULT_CLASS)]
    class_name: String,

    #[arg(long)]
    suite: Option<String>,
}

#[derive(Debug, Parser)]
struct SkipCommand {
    message: String,

    #[arg(long = "class", default_value = DEFAULT_CLASS)]
    class_name: String,

    #[arg(long)]
    suite: Option<String>,

    #[arg(long, help = "Use as the skipped testcase message")]
    details: Option<String>,
}

#[derive(Debug, Parser)]
struct IssueCommand {
    message: String,

    #[arg(long = "class", default_value = DEFAULT_CLASS)]
    class_name: String,

    #[arg(long)]
    suite: Option<String>,

    #[arg(long)]
    details: Option<String>,

    #[arg(long)]
    fatal: bool,
}

impl IssueCommand {
    fn into_parts(self) -> (String, AddOptions) {
        (
            self.message,
            AddOptions {
                suite: self.suite,
                class: self.class_name,
                details: self.details,
            },
        )
    }
}

#[derive(Debug, Parser)]
struct RunCommand {
    #[command(flatten)]
    issue: IssueCommand,

    #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<String>,
}

#[derive(Debug, Parser)]
struct FinishCommand {
    #[arg(
        long,
        value_enum,
        default_value_t = Platform::Auto,
        help = "Select native CI reporting output"
    )]
    platform: Platform,

    #[arg(long, help = "Also write a GitHub Actions job summary")]
    github_summary: bool,

    #[arg(long, help = "Also emit GitHub Actions error annotations")]
    github_annotations: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Platform {
    #[value(alias = "detect")]
    Auto,
    #[value(name = "github", alias = "gh", alias = "ghub")]
    GitHub,
    #[value(name = "gitlab", alias = "gl", alias = "glab")]
    GitLab,
    #[value(alias = "gt")]
    Generic,
    #[value(alias = "off")]
    None,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("tellci: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<u8> {
    let cli = Cli::parse();

    match cli.command {
        Command::Pass(command) => {
            tellci::pass(
                &cli.file,
                command.message,
                AddOptions {
                    suite: command.suite,
                    class: command.class_name,
                    details: None,
                },
            )?;
            Ok(0)
        }
        Command::Fail(command) => {
            let fatal = command.fatal;
            let (message, options) = command.into_parts();
            tellci::fail(&cli.file, message, options)?;
            Ok(if fatal { 1 } else { 0 })
        }
        Command::Error(command) => {
            let fatal = command.fatal;
            let (message, options) = command.into_parts();
            tellci::error(&cli.file, message, options)?;
            Ok(if fatal { 1 } else { 0 })
        }
        Command::Skip(command) => {
            tellci::skip(
                &cli.file,
                command.message,
                AddOptions {
                    suite: command.suite,
                    class: command.class_name,
                    details: command.details,
                },
            )?;
            Ok(0)
        }
        Command::Run(command) => {
            let exit_code = run_command(&cli.file, command)?;
            Ok(exit_code)
        }
        Command::Finish(command) => {
            let status = tellci::finish(&cli.file)?;
            let platform = resolve_platform(command.platform);
            let github = matches!(platform, Platform::GitHub);

            if github || command.github_summary {
                write_github_summary(&cli.file)?;
            }
            if github || command.github_annotations {
                print!("{}", tellci::github_annotations(&cli.file)?);
            }
            Ok(if status.is_success() { 0 } else { 1 })
        }
        Command::Reset { suite } => {
            tellci::reset(&cli.file, Some(&suite))?;
            Ok(0)
        }
        Command::Status => {
            let status = tellci::status(&cli.file)?;
            println!(
                "{}: {} tests, {} failures, {} errors, {} skipped",
                cli.file.display(),
                status.tests,
                status.failures,
                status.errors,
                status.skipped
            );
            Ok(0)
        }
    }
}

fn run_command(path: &Path, run: RunCommand) -> Result<u8> {
    let (program, args) = run
        .command
        .split_first()
        .context("missing command to run")?;
    let output = ProcessCommand::new(program).args(args).output();

    match output {
        Ok(output) if output.status.success() => {
            let (message, mut options) = run.issue.into_parts();
            options.details = None;
            tellci::pass(path, message, options)?;
            Ok(0)
        }
        Ok(output) => {
            let fatal = run.issue.fatal;
            let (message, mut options) = run.issue.into_parts();
            options.details = Some(run_details(
                &run.command,
                output.status.code(),
                &output.stdout,
                &output.stderr,
                options.details.as_deref(),
            ));
            tellci::fail(path, message, options)?;
            Ok(if fatal { 1 } else { 0 })
        }
        Err(error) => {
            let fatal = run.issue.fatal;
            let (message, mut options) = run.issue.into_parts();
            options.details = Some(format!(
                "Failed to start command `{}`: {error}",
                shell_words(&run.command)
            ));
            tellci::error(path, message, options)?;
            Ok(if fatal { 1 } else { 0 })
        }
    }
}

const MAX_STREAM_CHARS: usize = 8 * 1024;

fn run_details(
    command: &[String],
    exit_code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
    details: Option<&str>,
) -> String {
    let mut body = String::new();

    if let Some(details) = details {
        body.push_str(details);
        body.push_str("\n\n");
    }

    body.push_str(&format!("Command: {}\n", shell_words(command)));
    match exit_code {
        Some(exit_code) => body.push_str(&format!("Exit code: {exit_code}\n")),
        None => body.push_str("Exit code: terminated by signal\n"),
    }

    let stdout = truncate_stream(&String::from_utf8_lossy(stdout));
    if !stdout.is_empty() {
        body.push_str("\nstdout:\n");
        body.push_str(stdout.trim_end());
        body.push('\n');
    }

    let stderr = truncate_stream(&String::from_utf8_lossy(stderr));
    if !stderr.is_empty() {
        body.push_str("\nstderr:\n");
        body.push_str(stderr.trim_end());
        body.push('\n');
    }

    body
}

fn truncate_stream(stream: &str) -> String {
    let trimmed = stream.trim_end();
    if trimmed.chars().count() <= MAX_STREAM_CHARS {
        return stream.to_string();
    }

    let kept: String = trimmed.chars().take(MAX_STREAM_CHARS).collect();
    format!("{kept}\n[… output truncated]")
}

fn shell_words(command: &[String]) -> String {
    command
        .iter()
        .map(|part| {
            if part
                .chars()
                .all(|char| char.is_ascii_alphanumeric() || "-_./:=@".contains(char))
            {
                part.to_string()
            } else {
                format!("'{}'", part.replace('\'', "'\"'\"'"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn resolve_platform(platform: Platform) -> Platform {
    match platform {
        Platform::Auto => match CiProvider::detect() {
            CiProvider::GitHub => Platform::GitHub,
            CiProvider::GitLab => Platform::GitLab,
            CiProvider::Generic => Platform::Generic,
            CiProvider::Local => Platform::None,
        },
        platform => platform,
    }
}

fn write_github_summary(path: &Path) -> Result<()> {
    let markdown = tellci::github_markdown(path)?;

    if let Ok(summary_path) = env::var("GITHUB_STEP_SUMMARY") {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&summary_path)
            .with_context(|| format!("failed to open GitHub step summary file {summary_path}"))?;
        file.write_all(markdown.as_bytes())
            .with_context(|| format!("failed to write GitHub step summary file {summary_path}"))?;
    } else {
        println!("{markdown}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_streams_pass_through_unchanged() {
        assert_eq!(truncate_stream("hello\n"), "hello\n");
    }

    #[test]
    fn long_streams_are_truncated_with_marker() {
        let stream = "x".repeat(MAX_STREAM_CHARS + 100);
        let truncated = truncate_stream(&stream);

        assert!(truncated.starts_with(&"x".repeat(MAX_STREAM_CHARS)));
        assert!(truncated.ends_with("[… output truncated]"));
        assert!(truncated.len() < stream.len());
    }
}
