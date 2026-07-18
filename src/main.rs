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
mod index;
mod scanner;
mod search;
mod terminal;
mod tokenizer;

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{CommandFactory, Parser};

use crate::cli::{Cli, Command};
use crate::index::SearchIndex;
use crate::scanner::{ScannedDocument, scan_documents};
use crate::search::{MatchMode, search};
use crate::terminal::{color_enabled, render_index, render_search, render_startup};

/// This starts Shun, runs the selected command, prints its result.
/// Parameters: none
/// Returns: Ok(()) when the command completes successfully or an error when command execution fails.
fn main() -> Result<()> {
    let cli: Cli = Cli::parse();
    let color = color_enabled();

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
        }) => {
            let scanned_documents: Vec<ScannedDocument> = scan_documents(&directory)?;
            let index = SearchIndex::build(&scanned_documents);
            let mode = if match_all {
                MatchMode::All
            } else {
                MatchMode::Any
            };
            let results = search(&index, &query, mode);
            let report_directory = report_directory(&directory)?;
            write_output(&render_search(
                &report_directory,
                &query,
                mode,
                &index,
                &scanned_documents,
                &results,
                color,
            ))?;
        }
        None => {
            let mut help = Vec::new();
            Cli::command().write_help(&mut help)?;
            let help = String::from_utf8(help)?;
            write_output(&render_startup(&help, color))?;
        }
    }

    Ok(())
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
}
