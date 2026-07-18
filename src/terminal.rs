/* ============================================================================
 * Shun: terminal.rs
 * ============================================================================
 *
 * This file owns Shun's human-readable terminal.
 *
 * Color is emitted only for an interactive terminal and can be disabled with
 * the NO_COLOR environment variable. Redirected output remains plain text.
 *
 * ============================================================================
 */

use std::env;
use std::io::{self, IsTerminal};
use std::path::Path;

use term_table::row::Row;
use term_table::table_cell::{Alignment, TableCell};
use term_table::{Table, TableStyle};
use textwrap::Options;

use crate::classifier::FileCategory;
use crate::index::SearchIndex;
use crate::scanner::ScannedDocument;
use crate::search::{MatchMode, SearchResult, snippet_for};
use crate::tokenizer::tokenize;

const BANNER: &str = r" __ _
/ _\ |__  _   _ _ __
\ \| '_ \| | | | '_ \
_\ \ | | | |_| | | | |
\__/_| |_|\__,_|_| |_|";
const SEPARATOR: &str = "------------------------------------------------------------";
const SEARCH_TABLE_COLUMN_WIDTH: usize = 40;
const SEARCH_TABLE_CONTENT_WIDTH: usize = SEARCH_TABLE_COLUMN_WIDTH * 2 - 1;

const RESET: &str = "\x1b[0m";
const BOLD_CYAN: &str = "\x1b[1;36m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const DIM: &str = "\x1b[2m";

/// This determines whether human-readable output should include ANSI colors.
/// Parameters: none. Standard output and conventional color environment values are inspected.
/// Returns: true only for an interactive, color-capable terminal without NO_COLOR.
pub(crate) fn color_enabled() -> bool {
    color_enabled_for(
        io::stdout().is_terminal(),
        env::var_os("NO_COLOR").is_some(),
        env::var("TERM").ok().as_deref(),
    )
}

/// This renders Shun's startup identity above generated command help.
/// Parameters: help is Clap-generated text and color controls ANSI styling.
/// Returns: A complete startup screen ending in one newline.
pub(crate) fn render_startup(help: &str, color: bool) -> String {
    let mut output = String::new();
    output.push_str(&paint(BANNER, BOLD_CYAN, color));
    output.push_str("\n\n");
    output.push_str(&paint("Search code and documentation.", DIM, color));
    output.push('\n');
    output.push_str(&paint(SEPARATOR, DIM, color));
    output.push('\n');
    output.push_str(help.trim_end());
    output.push('\n');
    output
}

/// This renders indexed documents and corpus statistics as an aligned terminal report.
/// Parameters: directory identifies the repository, index contains documents and statistics,
/// and color controls ANSI styling.
/// Returns: A complete report ending in one newline.
pub(crate) fn render_index(directory: &Path, index: &SearchIndex, color: bool) -> String {
    let id_width = index
        .documents
        .last()
        .map_or(2, |document| document.id.to_string().len().max(2));
    let path_width = index
        .documents
        .iter()
        .map(|document| document.relative_path.display().to_string().chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let category_width = index
        .documents
        .iter()
        .map(|document| document.category.to_string().chars().count())
        .max()
        .unwrap_or(8)
        .max(8);
    let token_width = index
        .documents
        .iter()
        .map(|document| format_count(document.token_count).len())
        .max()
        .unwrap_or(6)
        .max(6);
    let table_width = id_width + path_width + category_width + token_width + 8;
    let separator = "-".repeat(table_width.max(SEPARATOR.len()));

    let mut output = String::new();
    push_heading(&mut output, "INDEX", color);
    output.push_str("Repository  ");
    output.push_str(&paint(&directory.display().to_string(), CYAN, color));
    output.push('\n');
    output.push_str(&paint(&separator, DIM, color));
    output.push('\n');
    output.push_str(&format!(
        "  {:>id_width$}  {:<path_width$}  {:<category_width$}  {:>token_width$}\n",
        "ID", "PATH", "CATEGORY", "TOKENS"
    ));
    output.push_str(&paint(&separator, DIM, color));
    output.push('\n');

    for document in &index.documents {
        let id = format!("{:>id_width$}", document.id);
        let path = format!(
            "{:<path_width$}",
            document.relative_path.display().to_string()
        );
        let category = format!("{:<category_width$}", document.category);
        let tokens = format!("{:>token_width$}", format_count(document.token_count));

        output.push_str("  ");
        output.push_str(&paint(&id, DIM, color));
        output.push_str("  ");
        output.push_str(&paint(&path, CYAN, color));
        output.push_str("  ");
        output.push_str(&paint(&category, YELLOW, color));
        output.push_str("  ");
        output.push_str(&paint(&tokens, GREEN, color));
        output.push('\n');
    }

    output.push_str(&paint(&separator, DIM, color));
    output.push('\n');
    push_heading(&mut output, "SUMMARY", color);
    push_metric(&mut output, "Files", index.documents.len(), color);
    push_metric(
        &mut output,
        "Unique terms",
        index.document_frequency.len(),
        color,
    );
    let posting_count: usize = index.postings.values().map(Vec::len).sum();
    push_metric(&mut output, "Posting entries", posting_count, color);
    push_metric(&mut output, "Source tokens", index.total_token_count, color);
    output.push_str(&format!(
        "  {:<18} {}\n",
        "Average length",
        paint(
            &format!("{:.2} tokens", index.average_document_length),
            GREEN,
            color
        )
    ));

    output.push('\n');
    push_heading(&mut output, "BY CATEGORY", color);
    for category in FileCategory::ALL {
        let count = index
            .documents
            .iter()
            .filter(|document| document.category == category)
            .count();
        if count > 0 {
            output.push_str(&format!(
                "  {:<18} {}\n",
                category,
                paint(&format_count(count), GREEN, color)
            ));
        }
    }

    output
}

/// This renders query context, ranked matches, and line-aware snippets.
/// Parameters: directory and query identify the request, mode describes OR or AND behavior,
/// index and scanned_documents resolve result metadata, results are ranked, and color controls ANSI.
/// Returns: A complete report ending in one newline.
pub(crate) fn render_search(
    directory: &Path,
    query: &str,
    mode: MatchMode,
    index: &SearchIndex,
    scanned_documents: &[ScannedDocument],
    results: &[SearchResult],
    color: bool,
) -> String {
    let mut output = String::new();
    push_heading(&mut output, "SEARCH", color);
    output.push_str("Query       ");
    output.push_str(&paint(&format!("\"{query}\""), CYAN, color));
    output.push('\n');
    output.push_str("Match       ");
    output.push_str(&paint(
        match mode {
            MatchMode::Any => "ANY term (OR)",
            MatchMode::All => "ALL terms (AND)",
        },
        MAGENTA,
        color,
    ));
    output.push('\n');
    output.push_str("Repository  ");
    output.push_str(&paint(&directory.display().to_string(), CYAN, color));
    output.push('\n');
    output.push_str(&paint(SEPARATOR, DIM, color));
    output.push('\n');

    if tokenize(query).source_token_count == 0 {
        output.push_str(&paint("No searchable terms in the query.", YELLOW, color));
        output.push('\n');
        output.push_str(&paint(
            "Use letters, numbers, underscores, or internal hyphens.",
            DIM,
            color,
        ));
        output.push('\n');
        return output;
    }

    if results.is_empty() {
        output.push_str(&paint("No matches found.", YELLOW, color));
        output.push('\n');
        let hint = match mode {
            MatchMode::Any => "Check the spelling or try a broader technical term.",
            MatchMode::All => "Try fewer terms or omit --match-all.",
        };
        output.push_str(&paint(hint, DIM, color));
        output.push('\n');
        return output;
    }

    output.push('\n');
    output.push_str(&render_search_results_table(
        index,
        scanned_documents,
        results,
        color,
    ));
    output
}

/// This renders ranked matches grouped by repository role with scores.
/// Parameters: index and scanned_documents resolve result metadata, results are ranked,
/// and color controls ANSI styling within cells.
/// Returns: A complete ASCII table ending in one newline.
fn render_search_results_table(
    index: &SearchIndex,
    scanned_documents: &[ScannedDocument],
    results: &[SearchResult],
    color: bool,
) -> String {
    let match_label = if results.len() == 1 {
        "1 match".to_owned()
    } else {
        format!("{} matches", format_count(results.len()))
    };
    let mut rows = vec![Row::new(vec![
        TableCell::builder(paint(
            &format!("SEARCH RESULTS ({match_label})"),
            BOLD_CYAN,
            color,
        ))
        .col_span(2)
        .alignment(Alignment::Center)
        .build(),
    ])];

    let mut result_ranks = vec![0; index.documents.len()];
    for (rank, result) in results.iter().enumerate() {
        result_ranks[result.document_id] = rank + 1;
    }

    for category in FileCategory::ALL {
        let category_results: Vec<&SearchResult> = results
            .iter()
            .filter(|result| index.documents[result.document_id].category == category)
            .collect();
        if category_results.is_empty() {
            continue;
        }

        rows.push(Row::new(vec![
            TableCell::builder(paint(
                &format!(
                    "{} ({})",
                    result_group_label(category),
                    format_count(category_results.len())
                ),
                YELLOW,
                color,
            ))
            .col_span(2)
            .alignment(Alignment::Center)
            .build(),
        ]));

        for result in category_results {
            let document = &index.documents[result.document_id];
            let scanned_document = &scanned_documents[result.document_id];
            let rank = result_ranks[result.document_id];
            let snippet = snippet_for(&scanned_document.content, result.line)
                .expect("posting lines must resolve within scanned content");

            rows.push(Row::new(vec![
                TableCell::builder(paint(
                    &format!("#{rank}  {}", document.relative_path.display()),
                    BOLD_CYAN,
                    color,
                ))
                .col_span(2)
                .alignment(Alignment::Center)
                .build(),
            ]));
            rows.push(Row::new(vec![
                TableCell::builder(format!(
                    "Line: {}",
                    paint(&result.line.to_string(), CYAN, color)
                ))
                .build(),
                TableCell::builder(format!(
                    "Category: {}  Score: {}",
                    paint(&document.category.to_string(), YELLOW, color),
                    paint(&format_score(result.score), GREEN, color)
                ))
                .alignment(Alignment::Right)
                .build(),
            ]));
            rows.push(Row::without_separator(vec![
                TableCell::builder(format!(
                    "Matches  {}",
                    paint(&result.matched_terms.join(", "), MAGENTA, color)
                ))
                .col_span(2)
                .build(),
            ]));
            rows.push(Row::without_separator(vec![
                TableCell::builder(paint(&format_score_factors(result), DIM, color))
                    .col_span(2)
                    .build(),
            ]));
            rows.push(Row::without_separator(vec![
                TableCell::builder(format_snippet(snippet))
                    .col_span(2)
                    .build(),
            ]));
        }
    }

    Table::builder()
        .max_column_width(SEARCH_TABLE_COLUMN_WIDTH)
        .style(TableStyle::simple())
        .rows(rows)
        .build()
        .render()
}

fn result_group_label(category: FileCategory) -> &'static str {
    match category {
        FileCategory::SourceCode => "IMPLEMENTATION",
        FileCategory::Documentation => "DOCUMENTATION",
        FileCategory::Configuration => "CONFIGURATION",
        FileCategory::Test => "TESTS",
        FileCategory::Example => "EXAMPLES",
        FileCategory::ProjectMetadata => "PROJECT METADATA",
        FileCategory::Unknown => "OTHER",
    }
}

fn format_score_factors(result: &SearchResult) -> String {
    format!(
        "Factors: BM25 {} + file name {} + path {} + category {}",
        format_score(result.score_factors.bm25),
        format_score(result.score_factors.file_name_boost),
        format_score(result.score_factors.path_boost),
        format_score(result.score_factors.category_boost)
    )
}

fn format_snippet(snippet: &str) -> String {
    textwrap::fill(
        snippet,
        Options::new(SEARCH_TABLE_CONTENT_WIDTH)
            .initial_indent("Snippet: ")
            .subsequent_indent(""),
    )
}

/// This isolates terminal and environment checks for deterministic tests.
fn color_enabled_for(is_terminal: bool, no_color: bool, term: Option<&str>) -> bool {
    is_terminal && !no_color && term != Some("dumb")
}

/// This wraps text in one ANSI style only when color output is enabled.
fn paint(text: &str, style: &str, color: bool) -> String {
    if color {
        format!("{style}{text}{RESET}")
    } else {
        text.to_owned()
    }
}

/// This appends a compact uppercase section heading.
fn push_heading(output: &mut String, heading: &str, color: bool) {
    output.push_str(&paint(heading, BOLD_CYAN, color));
    output.push('\n');
}

/// This appends one aligned integer metric with thousands separators.
fn push_metric(output: &mut String, label: &str, value: usize, color: bool) {
    output.push_str(&format!(
        "  {label:<18} {}\n",
        paint(&format_count(value), GREEN, color)
    ));
}

/// This inserts comma group separators into a non-negative integer.
fn format_count(value: usize) -> String {
    let digits = value.to_string();
    let mut output = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            output.push(',');
        }
        output.push(digit);
    }

    output
}

fn format_score(value: f64) -> String {
    format!("{value:.3}")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::classifier::FileCategory;
    use crate::scanner::ScannedDocument;
    use crate::search::search;
    use crate::tokenizer::tokenize;

    use super::*;

    /// This creates a scanned document for terminal report tests.
    /// Parameters: path identifies the fixture and content supplies searchable text.
    /// Returns: A source-code document with retained content and positioned terms.
    fn scanned_document(path: &str, content: &str) -> ScannedDocument {
        let tokenized = tokenize(content);

        ScannedDocument {
            absolute_path: Path::new("/").join(path),
            relative_path: PathBuf::from(path),
            category: FileCategory::SourceCode,
            content: content.to_owned(),
            token_count: tokenized.source_token_count,
            tokens: tokenized.tokens,
        }
    }

    #[test]
    fn renders_plain_startup_banner() {
        let output = render_startup("Usage: shun <COMMAND>\n", false);

        assert!(output.starts_with(BANNER));
        assert!(output.contains("Search code and documentation."));
        assert!(output.contains(SEPARATOR));
        assert!(output.ends_with("Usage: shun <COMMAND>\n"));
        assert!(!output.contains("\x1b["));
    }

    #[test]
    fn enables_color_only_for_supported_interactive_terminals() {
        assert!(color_enabled_for(true, false, Some("xterm-256color")));
        assert!(!color_enabled_for(false, false, Some("xterm-256color")));
        assert!(!color_enabled_for(true, true, Some("xterm-256color")));
        assert!(!color_enabled_for(true, false, Some("dumb")));
        assert!(render_startup("help", true).contains("\x1b[1;36m"));
    }

    #[test]
    fn formats_terminal_counts() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1,000");
        assert_eq!(format_count(1_234_567), "1,234,567");
    }

    #[test]
    fn renders_plain_index_report() {
        let documents = vec![scanned_document("src/main.rs", "search index")];
        let index = SearchIndex::build(&documents);

        let output = render_index(Path::new("."), &index, false);

        assert!(output.starts_with("INDEX\nRepository  .\n"));
        assert!(output.contains("ID  PATH"));
        assert!(output.contains("src/main.rs"));
        assert!(output.contains("SUMMARY\n"));
        assert!(output.contains("Unique terms"));
        assert!(output.contains("BY CATEGORY\n"));
        assert!(!output.contains("\x1b["));
    }

    #[test]
    fn renders_search_results_and_empty_term_guidance() {
        let documents = vec![scanned_document(
            "src/cli.rs",
            "* It uses clap to parse arguments, generate help messages, and then check",
        )];
        let index = SearchIndex::build(&documents);
        let results = search(&index, "help", MatchMode::Any);

        let output = render_search(
            Path::new("."),
            "help",
            MatchMode::Any,
            &index,
            &documents,
            &results,
            false,
        );

        assert!(output.contains("SEARCH\nQuery       \"help\""));
        assert!(output.contains("Match       ANY term (OR)"));
        assert!(output.contains("SEARCH RESULTS (1 match)"));
        assert!(output.contains("IMPLEMENTATION (1)"));
        assert!(output.contains("+"));
        assert!(output.contains("|"));
        assert!(output.contains("#1  src/cli.rs"));
        assert!(output.contains("Line: 1"));
        assert!(output.contains("Category: Source code  Score: 0."));
        assert!(output.contains("Matches  help"));
        assert!(output.contains("Factors: BM25 0."));
        assert!(output.contains("file name 0.000 + path 0.000 + category 0.300"));
        assert!(output.contains("Snippet: * It uses clap"));
        assert!(!output.contains("ch |\n| eck"));
        assert!(output.lines().any(|line| line.contains("check")));
        assert!(output.lines().all(|line| line.chars().count() <= 83));

        let punctuation_output = render_search(
            Path::new("."),
            "???",
            MatchMode::Any,
            &index,
            &documents,
            &[],
            false,
        );
        assert!(punctuation_output.contains("No searchable terms in the query."));
    }

    #[test]
    fn groups_search_results_by_repository_role() {
        let mut documents = vec![
            scanned_document("src/search.rs", "search"),
            scanned_document("README.md", "search search search search"),
            scanned_document("config/shun.toml", "search search"),
        ];
        documents[1].category = FileCategory::Documentation;
        documents[2].category = FileCategory::Configuration;
        let index = SearchIndex::build(&documents);
        let results = search(&index, "search", MatchMode::Any);

        let output = render_search(
            Path::new("."),
            "search",
            MatchMode::Any,
            &index,
            &documents,
            &results,
            false,
        );

        let implementation = output.find("IMPLEMENTATION (1)").unwrap();
        let documentation = output.find("DOCUMENTATION (1)").unwrap();
        let configuration = output.find("CONFIGURATION (1)").unwrap();
        assert!(implementation < documentation);
        assert!(documentation < configuration);
        for document in &index.documents {
            let rank = results
                .iter()
                .position(|result| result.document_id == document.id)
                .unwrap()
                + 1;
            assert!(output.contains(&format!("#{rank}  {}", document.relative_path.display())));
        }
    }
}
