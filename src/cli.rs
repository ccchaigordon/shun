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

use clap::{Parser, Subcommand, ValueEnum};

use crate::classifier::FileCategory;

#[derive(Debug, Parser)]
#[command(
    name = "shun",
    version,
    about = "Search repository files and check documentation references"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Build an in-memory index and report repository statistics.
    Index {
        #[arg(help = "Repository directory to scan")]
        directory: PathBuf,
    },
    /// Search repository content with normalized developer-aware terms.
    Search {
        #[arg(help = "Query text to normalize and search")]
        query: String,
        #[arg(
            short,
            long,
            default_value = ".",
            help = "Repository directory to search"
        )]
        directory: PathBuf,
        #[arg(long, help = "Require every source query token to match")]
        match_all: bool,
        #[arg(
            long = "category",
            value_name = "CATEGORY",
            value_enum,
            value_delimiter = ',',
            help = "Keep only these repository roles. Values: source-code, documentation, configuration, tests, examples, project-metadata, unknown. Repeat or separate values with commas"
        )]
        categories: Vec<CategoryFilter>,
        #[arg(
            long = "path",
            value_name = "TEXT",
            help = "Keep only paths containing this text without case sensitivity"
        )]
        path_filter: Option<String>,
        #[arg(
            long = "extension",
            value_name = "EXTENSION",
            value_delimiter = ',',
            help = "Keep only these file extensions. Repeat or separate values with commas"
        )]
        extensions: Vec<String>,
        #[arg(
            long,
            value_name = "COUNT",
            value_parser = parse_positive_usize,
            help = "Return at most this many globally ranked results"
        )]
        limit: Option<usize>,
    },
    /// Find exact Rust symbol definitions and likely text references.
    Symbol {
        #[arg(help = "Case-sensitive Rust symbol name")]
        name: String,
        #[arg(
            short,
            long,
            default_value = ".",
            help = "Repository directory to inspect"
        )]
        directory: PathBuf,
    },
    /// Summarize project type, entry points, modules, and repository roles.
    Overview {
        #[arg(
            short,
            long,
            default_value = ".",
            help = "Repository directory to summarize"
        )]
        directory: PathBuf,
    },
}

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| "must be a positive integer".to_owned())
        .and_then(|value| {
            (value > 0)
                .then_some(value)
                .ok_or_else(|| "must be greater than zero".to_owned())
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum CategoryFilter {
    #[value(alias = "source")]
    SourceCode,
    #[value(alias = "docs")]
    Documentation,
    Configuration,
    #[value(alias = "test")]
    Tests,
    #[value(alias = "example")]
    Examples,
    #[value(alias = "metadata")]
    ProjectMetadata,
    Unknown,
}

impl From<CategoryFilter> for FileCategory {
    fn from(category: CategoryFilter) -> Self {
        match category {
            CategoryFilter::SourceCode => Self::SourceCode,
            CategoryFilter::Documentation => Self::Documentation,
            CategoryFilter::Configuration => Self::Configuration,
            CategoryFilter::Tests => Self::Test,
            CategoryFilter::Examples => Self::Example,
            CategoryFilter::ProjectMetadata => Self::ProjectMetadata,
            CategoryFilter::Unknown => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_composable_search_filters() {
        let cli = Cli::try_parse_from([
            "shun",
            "search",
            "ranking",
            "--category",
            "source,documentation",
            "--category",
            "tests",
            "--extension",
            "rs,md",
            "--path",
            "src",
            "--limit",
            "5",
        ])
        .unwrap();

        let Some(Command::Search {
            categories,
            path_filter,
            extensions,
            limit,
            ..
        }) = cli.command
        else {
            panic!("expected search command");
        };

        assert_eq!(
            categories,
            [
                CategoryFilter::SourceCode,
                CategoryFilter::Documentation,
                CategoryFilter::Tests
            ]
        );
        assert_eq!(path_filter.as_deref(), Some("src"));
        assert_eq!(extensions, ["rs", "md"]);
        assert_eq!(limit, Some(5));
    }

    #[test]
    fn rejects_a_zero_result_limit() {
        assert!(Cli::try_parse_from(["shun", "search", "ranking", "--limit", "0"]).is_err());
    }

    #[test]
    fn parses_symbol_lookup_directory() {
        let cli = Cli::try_parse_from([
            "shun",
            "symbol",
            "SearchIndex",
            "--directory",
            "another-project",
        ])
        .unwrap();

        assert!(matches!(
            cli.command,
            Some(Command::Symbol { name, directory })
                if name == "SearchIndex"
                    && directory == std::path::Path::new("another-project")
        ));
    }

    #[test]
    fn parses_overview_default_directory() {
        let cli = Cli::try_parse_from(["shun", "overview"]).unwrap();

        assert!(matches!(
            cli.command,
            Some(Command::Overview { directory }) if directory == std::path::Path::new(".")
        ));
    }
}
