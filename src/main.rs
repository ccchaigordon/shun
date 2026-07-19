/* ============================================================================
 * Shun: main.rs
 * ============================================================================
 *
 * This file parses command-line arguments into the types defined by the cli
 * module. It also selects the requested operation and delegates human-readable
 * output formatting to the terminal module.
 *
 * When no subcommand is supplied, it prints the generated help text and exits
 * successfully. Feature-specific work is done on modules like scanner.
 *
 * ============================================================================
 */

mod classifier;
mod cli;
mod documentation;
mod index;
mod overview;
mod pathing;
mod related;
mod scanner;
mod search;
mod symbol;
mod terminal;
mod tokenizer;

use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{CommandFactory, Parser};

use crate::cli::{Cli, Command};
use crate::documentation::DocumentationAudit;
use crate::index::SearchIndex;
use crate::overview::ProjectOverview;
use crate::related::find_related_files;
use crate::scanner::{ScannedDocument, scan_documents};
use crate::search::{MatchMode, SearchFilters, filter_results, search};
use crate::terminal::{
    SearchReport, color_enabled, render_audit, render_command_help, render_index, render_overview,
    render_related, render_search, render_startup, render_symbol,
};

enum HelpTarget {
    Root,
    Subcommand(String),
}

/// This starts Shun, runs the selected command, prints its result.
/// Parameters: none
/// Returns: Ok(()) when the command completes successfully or an error when command execution fails.
fn main() -> Result<()> {
    let color = color_enabled();
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if let Some(help) = requested_help(&arguments, color) {
        write_output(&help)?;
        return Ok(());
    }
    let cli: Cli = Cli::parse();

    match cli.command {
        Some(Command::Index { directory }) => {
            let scanned_documents: Vec<ScannedDocument> = scan_documents(&directory)?;
            let index = SearchIndex::build(&scanned_documents);
            let report_directory = report_directory(&directory)?;
            write_output(&render_index(&report_directory, &index, color))?;
        }
        Some(Command::Search {
            query,
            directory,
            match_all,
            categories,
            path_filter,
            extensions,
            limit,
        }) => {
            let scanned_documents: Vec<ScannedDocument> = scan_documents(&directory)?;
            let index = SearchIndex::build(&scanned_documents);
            let mode = if match_all {
                MatchMode::All
            } else {
                MatchMode::Any
            };
            let filters = SearchFilters::new(
                categories.into_iter().map(Into::into),
                path_filter,
                extensions,
                limit,
            );
            let results = filter_results(&index, search(&index, &query, mode), &filters);
            let report_directory = report_directory(&directory)?;
            write_output(&render_search(
                SearchReport {
                    directory: &report_directory,
                    query: &query,
                    mode,
                    index: &index,
                    scanned_documents: &scanned_documents,
                    results: &results,
                    filters: &filters,
                },
                color,
            ))?;
        }
        Some(Command::Symbol { name, directory }) => {
            let scanned_documents = scan_documents(&directory)?;
            let index = SearchIndex::build(&scanned_documents);
            let lookup = index.symbols.lookup(&scanned_documents, &name);
            let report_directory = report_directory(&directory)?;
            write_output(&render_symbol(
                &report_directory,
                &lookup,
                &index,
                &scanned_documents,
                color,
            ))?;
        }
        Some(Command::Overview { directory }) => {
            let scanned_documents = scan_documents(&directory)?;
            let index = SearchIndex::build(&scanned_documents);
            let overview = ProjectOverview::build(&scanned_documents);
            let report_directory = report_directory(&directory)?;
            write_output(&render_overview(
                &report_directory,
                &overview,
                &index,
                color,
            ))?;
        }
        Some(Command::Related {
            path,
            directory,
            limit,
        }) => {
            let scanned_documents = scan_documents(&directory)?;
            let index = SearchIndex::build(&scanned_documents);
            let mut related =
                find_related_files(&scanned_documents, &index, &path).ok_or_else(|| {
                    anyhow::anyhow!("file is not in the scanned repository: {}", path.display())
                })?;
            related.results.truncate(limit);
            let report_directory = report_directory(&directory)?;
            write_output(&render_related(&report_directory, &related, &index, color))?;
        }
        Some(Command::AuditDocs {
            directory,
            confidence,
        }) => {
            let scanned_documents = scan_documents(&directory)?;
            let index = SearchIndex::build(&scanned_documents);
            let mut command = Cli::command();
            command.build();
            let mut audit = DocumentationAudit::build(&scanned_documents, &index, &command);
            audit
                .findings
                .retain(|finding| confidence.includes(finding.confidence));
            let report_directory = report_directory(&directory)?;
            write_output(&render_audit(
                &report_directory,
                &audit,
                &index,
                &scanned_documents,
                confidence,
                color,
            ))?;
        }
        None => {
            write_output(&render_startup(&Cli::command(), color))?;
        }
    }

    Ok(())
}

fn requested_help(arguments: &[OsString], color: bool) -> Option<String> {
    let target = requested_help_target(arguments)?;
    let mut command = Cli::command();
    command.build();

    match target {
        HelpTarget::Root => Some(render_startup(&command, color)),
        HelpTarget::Subcommand(name) => command
            .find_subcommand(&name)
            .map(|subcommand| render_command_help(subcommand, color)),
    }
}

fn requested_help_target(arguments: &[OsString]) -> Option<HelpTarget> {
    if arguments
        .first()
        .is_some_and(|value| value == OsStr::new("help"))
    {
        return Some(
            arguments
                .get(1)
                .and_then(|value| value.to_str())
                .map(|name| HelpTarget::Subcommand(name.to_owned()))
                .unwrap_or(HelpTarget::Root),
        );
    }

    arguments
        .iter()
        .take_while(|value| *value != OsStr::new("--"))
        .any(|value| value == OsStr::new("--help") || value == OsStr::new("-h"))
        .then(|| {
            arguments
                .first()
                .and_then(|value| value.to_str())
                .filter(|value| !value.starts_with('-'))
                .map(|name| HelpTarget::Subcommand(name.to_owned()))
                .unwrap_or(HelpTarget::Root)
        })
}

/// This writes one complete rendered report to standard output.
/// Parameters: output is a fully formatted terminal buffer.
/// Returns: Ok when all bytes are written, or an I/O error from standard output.
fn write_output(output: &str) -> io::Result<()> {
    io::stdout().lock().write_all(output.as_bytes())
}

/// This resolves the default dot directory for display in terminal reports.
/// Parameters: directory is the path supplied by the CLI.
/// Returns: The current working directory for dot, or the supplied path unchanged.
fn report_directory(directory: &Path) -> io::Result<PathBuf> {
    if directory == Path::new(".") {
        std::env::current_dir()
    } else {
        Ok(directory.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_dot_for_terminal_reports() -> io::Result<()> {
        assert_eq!(report_directory(Path::new("."))?, std::env::current_dir()?);
        assert_eq!(
            report_directory(Path::new("another-project"))?,
            PathBuf::from("another-project")
        );
        Ok(())
    }

    #[test]
    fn detects_root_and_subcommand_help_requests() {
        assert!(matches!(
            requested_help_target(&[OsString::from("--help")]),
            Some(HelpTarget::Root)
        ));
        assert!(matches!(
            requested_help_target(&[OsString::from("search"), OsString::from("-h")]),
            Some(HelpTarget::Subcommand(name)) if name == "search"
        ));
        assert!(matches!(
            requested_help_target(&[OsString::from("help"), OsString::from("index")]),
            Some(HelpTarget::Subcommand(name)) if name == "index"
        ));
        assert!(
            requested_help_target(&[
                OsString::from("search"),
                OsString::from("--"),
                OsString::from("--help")
            ])
            .is_none()
        );
    }
}
