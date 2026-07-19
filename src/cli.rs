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

use std::fmt;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::classifier::FileCategory;
use crate::documentation::Confidence;

#[derive(Debug, Parser)]
#[command(
    name = "shun",
    version,
    about = "Search repository files and check documentation references"
)]
pub(crate) struct Cli {
    #[arg(
        long,
        global = true,
        value_enum,
        default_value_t = OutputFormat::Human,
        help = "Select human-readable or structured JSON output"
    )]
    pub(crate) format: OutputFormat,
    #[arg(
        long,
        global = true,
        help = "Disable ANSI colors in human-readable output"
    )]
    pub(crate) no_color: bool,
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum OutputFormat {
    Human,
    Json,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Human => "human",
            Self::Json => "json",
        })
    }
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Build and save an index, then report repository statistics.
    Index {
        #[arg(help = "Repository directory to scan")]
        directory: PathBuf,
        #[arg(
            long = "exclude",
            value_name = "DIRECTORY",
            value_delimiter = ',',
            help = "Exclude directory names. Repeat or separate values with commas"
        )]
        exclusions: Vec<String>,
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
    /// Find tests, documentation, callers, and configuration related to a file.
    Related {
        #[arg(help = "Repository-relative file path to inspect")]
        path: PathBuf,
        #[arg(
            short,
            long,
            default_value = ".",
            help = "Repository directory to inspect"
        )]
        directory: PathBuf,
        #[arg(
            long,
            default_value_t = 20,
            value_parser = parse_positive_usize,
            help = "Return at most this many related files"
        )]
        limit: usize,
    },
    /// Report potentially stale references in Markdown documentation.
    AuditDocs {
        #[arg(
            short,
            long,
            default_value = ".",
            help = "Repository directory to audit"
        )]
        directory: PathBuf,
        #[arg(
            long,
            value_enum,
            default_value_t = Confidence::Low,
            help = "Minimum confidence to report"
        )]
        confidence: Confidence,
    },
    /// Display statistics from a saved repository index.
    Stats {
        #[arg(
            short,
            long,
            default_value = ".",
            help = "Repository containing the saved index"
        )]
        directory: PathBuf,
    },
    /// Remove saved index data from a repository.
    Clear {
        #[arg(
            short,
            long,
            default_value = ".",
            help = "Repository whose saved index should be removed"
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

    #[test]
    fn parses_related_limit_and_audit_confidence() {
        let related =
            Cli::try_parse_from(["shun", "related", "src/index.rs", "--limit", "8"]).unwrap();
        assert!(matches!(
            related.command,
            Some(Command::Related { path, limit: 8, .. })
                if path == std::path::Path::new("src/index.rs")
        ));

        let audit = Cli::try_parse_from(["shun", "audit-docs", "--confidence", "medium"]).unwrap();
        assert!(matches!(
            audit.command,
            Some(Command::AuditDocs {
                confidence: Confidence::Medium,
                ..
            })
        ));
    }

    #[test]
    fn parses_index_exclusions_and_storage_commands() {
        let index = Cli::try_parse_from([
            "shun",
            "index",
            ".",
            "--exclude",
            "vendor,fixtures",
            "--exclude",
            "archive",
        ])
        .unwrap();
        assert!(matches!(
            index.command,
            Some(Command::Index { exclusions, .. })
                if exclusions == ["vendor", "fixtures", "archive"]
        ));
        assert!(matches!(
            Cli::try_parse_from(["shun", "stats"]).unwrap().command,
            Some(Command::Stats { .. })
        ));
        assert!(matches!(
            Cli::try_parse_from(["shun", "clear"]).unwrap().command,
            Some(Command::Clear { .. })
        ));
    }

    #[test]
    fn parses_global_non_interactive_controls() {
        let cli =
            Cli::try_parse_from(["shun", "search", "index", "--format", "json", "--no-color"])
                .unwrap();

        assert_eq!(cli.format, OutputFormat::Json);
        assert!(cli.no_color);
    }
}
