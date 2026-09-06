//! folio — reference CLI implementation of the Repofolio standard.
//! M1 scope: `check` and `registry`. See docs/planning/folio-plan.md
//! (Phase M1).
//!
//! `check` (M1 check-plan step 9, docs/archive/plans/2026-09-06-m1-check-plan.md)
//! wires `repofolio_core::check` onto the command line and grades the
//! process exit status: 0 when clean or warnings-only (so continuous
//! integration can annotate a build without failing it), 1 when at least
//! one error-severity finding is present, 2 on a usage or internal
//! failure. `init` and `add` come later in the same milestone, once
//! `check` has a vertical slice — this file does not implement them yet.
//!
//! `registry` (added after the original check-plan, per CLAUDE.md's
//! non-negotiable architecture decision 6) wires `repofolio_core::
//! Registry` onto the command line; see
//! docs/specification/folio-registry.md.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

use repofolio_core::{check, Location, Registry, Report, Severity};

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

    /// Dump the FOLIO- diagnostic code registry and the ecosystem
    /// registry as one document: the tool-contract clause CLAUDE.md
    /// requires of every orchestrated tool, folio included.
    Registry {
        /// Output format. Only `json` exists — a registry dump is
        /// consumed by agents and MCP, not read as prose.
        #[arg(long, value_enum, default_value = "json")]
        format: RegistryFormat,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Format {
    Human,
    Json,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum RegistryFormat {
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
        Command::Registry { format } => run_registry(format),
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

/// Dumps the `FOLIO-` code registry and the ecosystem registry as one
/// JSON document. Always exits 0 — a registry dump describes what
/// exists, so unlike `check` there is no finding to grade an exit
/// status from; a serialization failure would be an internal error
/// (the `Result`-based `Err` path in `main`), not a usage failure.
fn run_registry(format: RegistryFormat) -> Result<ExitCode> {
    let registry = Registry::new();

    match format {
        RegistryFormat::Json => {
            let json = serde_json::to_string_pretty(&registry)
                .context("failed to serialize the registry as JSON")?;
            println!("{json}");
        }
    }

    Ok(ExitCode::from(0))
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
            single_line(&diagnostic.message)
        );
    }

    println!(
        "{} {} ({} {}, {} {}, {} info)",
        report.count,
        plural(report.count, "finding"),
        report.by_severity.error,
        plural(report.by_severity.error, "error"),
        report.by_severity.warning,
        plural(report.by_severity.warning, "warning"),
        report.by_severity.info,
    );
}

/// Collapses `message` onto one line, joining any internal whitespace
/// runs (including newlines) with a single space. A `Diagnostic.message`
/// is not guaranteed single-line at the data-model level — `FOLIO-001`/
/// `FOLIO-003` can carry `toml_edit::de::Error`'s multi-line,
/// pretty-printed parse diagnostic (a source snippet plus a caret line)
/// verbatim, kept that way deliberately so `--format json` and a future
/// SARIF serializer still see the full detail. This is the one place
/// that needs exactly one line per finding, so it collapses at render
/// time instead of the data model discarding the detail for everyone.
fn single_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `singular` unchanged for a count of exactly 1, `singular` + "s"
/// otherwise. `info` is not pluralized this way — "0 info"/"1 info"
/// reads fine as-is, matching the JSON field name.
fn plural(count: usize, singular: &str) -> String {
    if count == 1 {
        singular.to_string()
    } else {
        format!("{singular}s")
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The `warnings_human`/`clean_human` acceptance snapshots only cover
    /// counts of 0 and 5 for "warning", both of which read correctly as
    /// plural regardless of whether `plural()`'s `count == 1` branch
    /// exists at all — a fixture that also pinned a count of exactly 1
    /// would need a new snapshot per counter. This unit test pins the
    /// boundary directly instead, for every counter `plural()` is
    /// actually called with.
    #[test]
    fn plural_uses_singular_only_for_exactly_one() {
        for singular in ["finding", "error", "warning"] {
            assert_eq!(plural(0, singular), format!("{singular}s"));
            assert_eq!(plural(1, singular), singular);
            assert_eq!(plural(2, singular), format!("{singular}s"));
            assert_eq!(plural(5, singular), format!("{singular}s"));
        }
    }

    /// `ParseError::Toml`'s `Display` (surfaced verbatim in
    /// `Diagnostic.message` for `FOLIO-003`) is multi-line by design —
    /// `single_line` is what keeps `print_human`'s one-line-per-finding
    /// promise, not anything upstream in `repofolio-core`.
    #[test]
    fn single_line_collapses_embedded_newlines_and_repeated_whitespace() {
        let multi_line = "TOML parse error at line 1, column 8\n  |\n1 | name = \n  |        ^\ninvalid string\nexpected `\"`, `'`";

        let collapsed = single_line(multi_line);

        assert!(!collapsed.contains('\n'));
        assert_eq!(
            collapsed,
            "TOML parse error at line 1, column 8 | 1 | name = | ^ invalid string expected `\"`, `'`"
        );
    }
}
