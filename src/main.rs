/* ============================================================================
 * Shun: main.rs
 * ============================================================================
 *
 * This file parses command-line arguments into the types defined by the cli
 * module. It also selects the requested operation and formats results for
 * terminal output.
 *
 * When no subcommand is supplied, it prints the generated help text and exits
 * successfully. Feature-specific work is done on modules like scanner.
 *
 * ============================================================================
 */

mod cli;
mod scanner;
mod tokenizer;

use anyhow::Result;
use clap::{CommandFactory, Parser};

use crate::cli::{Cli, Command};
use crate::scanner::{ScannedDocument, scan_documents};

/// This starts Shun, runs the selected command, prints its result.
/// Parameters: none
/// Returns: Ok(()) when the command completes successfully or an error when command execution fails.

fn main() -> Result<()> {
    let cli: Cli = Cli::parse();

    match cli.command {
        Some(Command::Index { directory }) => {
            let documents: Vec<ScannedDocument> = scan_documents(&directory)?;

            for document in &documents {
                println!(
                    "{}: {} tokens",
                    document.relative_path.display(),
                    document.token_count
                );
            }

            println!("Indexed {} documents.", documents.len());
        }
        None => {
            Cli::command().print_help()?;
            println!();
        }
    }

    Ok(())
}
