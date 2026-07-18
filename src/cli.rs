/* ============================================================================
 * Shun: cli.rs
 * ============================================================================
 *
 * This file defines Shun's command-line interface and available commands.
 *
 * It uses clap to parse arguments, generate help messages, and then check
 * that each command is structured correctly before the main application logic
 * runs.
 *
 * I prefer keeping the CLI definitions here to make my life easier to maintain
 * in the future.
 *
 * ============================================================================
 */

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "shun",
    version,
    about = "Search repositories and audit documentation with local, explainable evidence"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    Index { directory: PathBuf },
}
