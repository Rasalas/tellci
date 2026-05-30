use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tellci::{AddOptions, DEFAULT_CLASS, DEFAULT_FILE, DEFAULT_SUITE};

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
    Finish,

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
        Command::Finish => {
            let status = tellci::finish(&cli.file)?;
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
