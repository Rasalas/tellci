use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
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
    Fail(FailCommand),

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
struct FailCommand {
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
    #[value(name = "github", alias = "gh")]
    GitHub,
    #[value(name = "gitlab", alias = "gl")]
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
            tellci::fail(
                &cli.file,
                command.message,
                AddOptions {
                    suite: command.suite,
                    class: command.class_name,
                    details: command.details,
                },
            )?;

            Ok(if command.fatal { 1 } else { 0 })
        }
        Command::Finish(command) => {
            let status = tellci::finish(&cli.file)?;
            let platform = command.platform.detect();
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

impl Platform {
    fn detect(self) -> Self {
        match self {
            Self::Auto => match CiProvider::detect() {
                CiProvider::GitHub => Self::GitHub,
                CiProvider::GitLab => Self::GitLab,
                CiProvider::Generic => Self::Generic,
                CiProvider::Local => Self::None,
            },
            platform => platform,
        }
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
