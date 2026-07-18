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

mod classifier;
mod cli;
mod index;
mod scanner;
mod search;
mod tokenizer;

use anyhow::Result;
use clap::{CommandFactory, Parser};

use crate::classifier::FileCategory;
use crate::cli::{Cli, Command};
use crate::index::SearchIndex;
use crate::scanner::{ScannedDocument, scan_documents};
use crate::search::{MatchMode, search, snippet_for};

/// This starts Shun, runs the selected command, prints its result.
/// Parameters: none
/// Returns: Ok(()) when the command completes successfully or an error when command execution fails.
fn main() -> Result<()> {
    let cli: Cli = Cli::parse();

    match cli.command {
        Some(Command::Index { directory }) => {
            let scanned_documents: Vec<ScannedDocument> = scan_documents(&directory)?;
            let index = SearchIndex::build(&scanned_documents);

            for document in &index.documents {
                println!(
                    "{:>4}  {} [{}]: {} tokens",
                    document.id,
                    document.relative_path.display(),
                    document.category,
                    document.token_count
                );
            }

            println!("\nIndexed {} files.", index.documents.len());
            for category in FileCategory::ALL {
                let count = index
                    .documents
                    .iter()
                    .filter(|document| document.category == category)
                    .count();

                if count > 0 {
                    println!("  {category}: {count}");
                }
            }

            let posting_count: usize = index
                .document_frequency
                .keys()
                .map(|term| index.postings_for(term).len())
                .sum();
            println!("Unique terms: {}", index.document_frequency.len());
            println!("Posting entries: {posting_count}");
            println!("Total source tokens: {}", index.total_token_count);
            println!(
                "Average document length: {:.2} tokens",
                index.average_document_length
            );
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

            if results.is_empty() {
                println!("No matches found.");
            } else {
                for (rank, result) in results.iter().enumerate() {
                    let document = &index.documents[result.document_id];
                    let scanned_document = &scanned_documents[result.document_id];
                    let snippet = snippet_for(&scanned_document.content, result.line)
                        .expect("posting lines must resolve within scanned content");

                    println!(
                        "{}. {}:{} [{}] (score: {})",
                        rank + 1,
                        document.relative_path.display(),
                        result.line,
                        document.category,
                        result.score
                    );
                    println!("   {snippet}");
                }
            }
        }
        None => {
            Cli::command().print_help()?;
            println!();
        }
    }

    Ok(())
}
