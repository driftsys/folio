//! folio — reference CLI implementation of the Repofolio standard.
//! M1 scope: `check` + `init`. See docs/planning/folio-plan.md (Phase M1).
//!
//! `check` (M1 check-plan step 9, docs/wip/2026-09-06-m1-check-plan.md)
//! wires `repofolio_core::check` onto the command line and grades the
//! process exit status: 0 when clean or warnings-only (so continuous
//! integration can annotate a build without failing it), 1 when at least
//! one error-severity finding is present, 2 on a usage or internal
//! failure. `init` and `add` are out of scope for this milestone.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

use repofolio_core::{check, Location, Report, Severity};

#[derive(Parser)]
#[command(
    name = "folio",
    version,
    about = "Reference CLI for the Repofolio standard"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a repository's conformance to the Repofolio standard.
    Check {
        /// Repository root to check. Defaults to the current directory.
        path: Option<PathBuf>,

        /// Output format.
        #[arg(long, value_enum, default_value = "human")]
        format: Format,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Format {
    Human,
    Json,
}

fn main() -> ExitCode {
    match run() {
        Ok(exit_code) => exit_code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Command::Check { path, format } => run_check(path, format),
    }
}

/// Runs the check pipeline against `path` (the current directory when
/// absent) and reports it in `format`. The exit-status contract lives
/// here: 0 clean/warnings-only, 1 any error-severity finding, 2 a usage
/// failure such as a path that is not a directory.
fn run_check(path: Option<PathBuf>, format: Format) -> Result<ExitCode> {
    let repo_root = match path {
        Some(path) => path,
        None => std::env::current_dir().context("failed to determine the current directory")?,
    };

    if !repo_root.is_dir() {
        bail!("not a directory: {}", repo_root.display());
    }

    let report = check(&repo_root);

    match format {
        Format::Json => {
            let json = serde_json::to_string_pretty(&report)
                .context("failed to serialize the report as JSON")?;
            println!("{json}");
        }
        Format::Human => print_human(&report),
    }

    if report.by_severity.error > 0 {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::from(0))
    }
}

/// The default, human-readable report: one line per finding, then a
/// summary line with the total and the per-severity counts.
fn print_human(report: &Report) {
    for diagnostic in &report.diagnostics {
        println!(
            "{} {} {}: {}",
            severity_label(diagnostic.severity),
            diagnostic.code,
            location_label(&diagnostic.location),
            diagnostic.message
        );
    }

    println!(
        "{} findings ({} errors, {} warnings, {} info)",
        report.count, report.by_severity.error, report.by_severity.warning, report.by_severity.info
    );
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

/// `location.file`, with `:line` and `:line:column` appended when the
/// finding has them. No M1 rule populates `line`/`column` today, but
/// `Location` carries them for a future content-match rule, so this
/// prints them rather than silently dropping them.
fn location_label(location: &Location) -> String {
    match (location.line, location.column) {
        (Some(line), Some(column)) => format!("{}:{line}:{column}", location.file),
        (Some(line), None) => format!("{}:{line}", location.file),
        _ => location.file.clone(),
    }
}
