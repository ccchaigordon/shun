/* ============================================================================
 * Shun: documentation.rs
 * ============================================================================
 *
 * This file extracts code-like references from Markdown and validates them
 * against scanned paths, Rust symbols, Clap metadata, and configuration keys.
 *
 * Findings will identify potentially stale documentation rather than proving
 * that prose is incorrect.
 *
 * ============================================================================
 */

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use clap::{Command, ValueEnum};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use crate::classifier::FileCategory;
use crate::index::{DocumentId, SearchIndex};
use crate::pathing::normalize_repository_path;
use crate::scanner::ScannedDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub(crate) enum Confidence {
    High,
    Medium,
    Low,
}

impl fmt::Display for Confidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
        })
    }
}

impl Confidence {
    pub(crate) const ALL: [Self; 3] = [Self::High, Self::Medium, Self::Low];

    pub(crate) fn includes(self, confidence: Self) -> bool {
        confidence <= self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ReferenceKind {
    Path,
    Symbol,
    Command,
    Option,
    ConfigurationKey,
    Module,
}

impl fmt::Display for ReferenceKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Path => "File path",
            Self::Symbol => "Rust symbol",
            Self::Command => "CLI command",
            Self::Option => "CLI option",
            Self::ConfigurationKey => "Configuration key",
            Self::Module => "Rust module",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DocumentationReference {
    pub(crate) document_id: DocumentId,
    pub(crate) line: usize,
    pub(crate) kind: ReferenceKind,
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuditFinding {
    pub(crate) reference: DocumentationReference,
    pub(crate) confidence: Confidence,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocumentationAudit {
    pub(crate) documents_checked: usize,
    pub(crate) references_checked: usize,
    pub(crate) findings: Vec<AuditFinding>,
}

impl DocumentationAudit {
    pub(crate) fn build(
        documents: &[ScannedDocument],
        index: &SearchIndex,
        command: &Command,
    ) -> Self {
        let references = extract_references(documents, index);
        let known_paths = documents
            .iter()
            .map(|document| normalize_repository_path(&document.relative_path))
            .collect::<BTreeSet<_>>();
        let known_symbols = index
            .symbols
            .symbols
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect::<BTreeSet<_>>();
        let known_commands = command
            .get_subcommands()
            .map(clap::Command::get_name)
            .collect::<BTreeSet<_>>();
        let known_options = command_options(command);
        let known_configuration_keys = configuration_keys(documents);

        let findings = references
            .iter()
            .filter_map(|reference| {
                validate_reference(
                    reference,
                    documents,
                    &known_paths,
                    &known_symbols,
                    &known_commands,
                    &known_options,
                    &known_configuration_keys,
                )
            })
            .collect();

        Self {
            documents_checked: documents
                .iter()
                .filter(|document| document.category == FileCategory::Documentation)
                .count(),
            references_checked: references.len(),
            findings,
        }
    }
}

fn extract_references(
    documents: &[ScannedDocument],
    index: &SearchIndex,
) -> Vec<DocumentationReference> {
    let known_symbols = index
        .symbols
        .symbols
        .iter()
        .map(|symbol| symbol.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut references = BTreeSet::new();

    for (document_id, document) in documents.iter().enumerate().filter(|(_, document)| {
        document.category == FileCategory::Documentation
            && document
                .relative_path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    }) {
        let mut in_code_block = false;
        for (event, range) in Parser::new(&document.content).into_offset_iter() {
            let line = line_for_offset(&document.content, range.start);
            match event {
                Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
                Event::End(TagEnd::CodeBlock) => in_code_block = false,
                Event::Code(value) => {
                    if !should_ignore_line(&document.content, line) {
                        classify_code_reference(
                            document_id,
                            line,
                            &value,
                            true,
                            &known_symbols,
                            &mut references,
                        );
                    }
                }
                Event::Text(value) if in_code_block => {
                    for (line_offset, value) in value.lines().enumerate() {
                        let source_line = line + line_offset;
                        if !should_ignore_line(&document.content, source_line) {
                            classify_code_reference(
                                document_id,
                                source_line,
                                value,
                                false,
                                &known_symbols,
                                &mut references,
                            );
                        }
                    }
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    if let Some(path) = path_reference(&dest_url) {
                        references.insert(DocumentationReference {
                            document_id,
                            line,
                            kind: ReferenceKind::Path,
                            value: path,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    references.into_iter().collect()
}

fn classify_code_reference(
    document_id: DocumentId,
    line: usize,
    value: &str,
    inline: bool,
    known_symbols: &BTreeSet<&str>,
    references: &mut BTreeSet<DocumentationReference>,
) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }

    let words = value.split_whitespace().collect::<Vec<_>>();
    let invocation_start = shun_invocation_start(&words);
    if let Some(start) = invocation_start
        && let Some(command) = words.get(start).map(|word| trim_token(word))
        && !command.starts_with('-')
        && is_identifier(command)
    {
        insert_reference(
            references,
            document_id,
            line,
            ReferenceKind::Command,
            command,
        );
    }

    for (word_index, word) in words.into_iter().enumerate() {
        let function_symbol = word
            .trim_matches(|character: char| {
                matches!(
                    character,
                    '`' | '"' | '\'' | ',' | ';' | ':' | '.' | '[' | ']' | '{' | '}'
                )
            })
            .strip_suffix("()")
            .filter(|symbol| is_identifier(symbol));
        let word = trim_token(word);
        let is_shun_option = invocation_start.is_some_and(|start| word_index >= start);
        let is_standalone_option = inline && value.split_whitespace().count() == 1;
        if word.len() > 2
            && word.starts_with("--")
            && word[2..].chars().all(is_option_character)
            && (is_shun_option || is_standalone_option)
        {
            insert_reference(references, document_id, line, ReferenceKind::Option, word);
        }
        if let Some(path) = path_reference(word) {
            insert_reference(references, document_id, line, ReferenceKind::Path, &path);
        }
        if let Some(symbol) = function_symbol {
            insert_reference(references, document_id, line, ReferenceKind::Symbol, symbol);
        } else if known_symbols.contains(word) {
            insert_reference(references, document_id, line, ReferenceKind::Symbol, word);
        } else if word.contains("::") && word.split("::").all(is_identifier) {
            insert_reference(references, document_id, line, ReferenceKind::Module, word);
        }
    }

    if let Some((key, _)) = value.split_once('=') {
        let key = key.trim();
        if key.contains('_')
            && key
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_')
            && is_identifier(key)
        {
            insert_reference(
                references,
                document_id,
                line,
                ReferenceKind::ConfigurationKey,
                key,
            );
        }
    }
}

fn insert_reference(
    references: &mut BTreeSet<DocumentationReference>,
    document_id: DocumentId,
    line: usize,
    kind: ReferenceKind,
    value: &str,
) {
    references.insert(DocumentationReference {
        document_id,
        line,
        kind,
        value: value.to_owned(),
    });
}

fn validate_reference(
    reference: &DocumentationReference,
    documents: &[ScannedDocument],
    known_paths: &BTreeSet<String>,
    known_symbols: &BTreeSet<&str>,
    known_commands: &BTreeSet<&str>,
    known_options: &BTreeSet<String>,
    known_configuration_keys: &BTreeSet<String>,
) -> Option<AuditFinding> {
    let missing = match reference.kind {
        ReferenceKind::Path => {
            let document = &documents[reference.document_id];
            let root_path = normalize_reference_path(&reference.value);
            let local_path = document
                .relative_path
                .parent()
                .map(|parent| normalize_repository_path(&parent.join(&reference.value)));
            !known_paths.contains(&root_path)
                && !local_path.is_some_and(|path| known_paths.contains(&path))
        }
        ReferenceKind::Symbol => !known_symbols.contains(reference.value.as_str()),
        ReferenceKind::Command => !known_commands.contains(reference.value.as_str()),
        ReferenceKind::Option => !known_options.contains(&reference.value),
        ReferenceKind::ConfigurationKey => !known_configuration_keys.contains(&reference.value),
        ReferenceKind::Module => {
            let module = reference.value.rsplit("::").next().unwrap_or_default();
            !known_symbols.contains(module)
                && !known_paths
                    .iter()
                    .any(|path| path.ends_with(&format!("/{module}.rs")))
        }
    };
    if !missing {
        return None;
    }

    let (confidence, reason) = match reference.kind {
        ReferenceKind::Path => (Confidence::High, "No matching repository path exists"),
        ReferenceKind::Command => (Confidence::High, "No matching CLI command is defined"),
        ReferenceKind::Option => (Confidence::High, "No matching CLI option is defined"),
        ReferenceKind::Symbol => (Confidence::Medium, "No exact Rust symbol is indexed"),
        ReferenceKind::Module => (Confidence::Medium, "No matching Rust module is indexed"),
        ReferenceKind::ConfigurationKey => {
            (Confidence::Low, "No matching configuration key is indexed")
        }
    };

    Some(AuditFinding {
        reference: reference.clone(),
        confidence,
        reason: reason.to_owned(),
    })
}

fn command_options(command: &Command) -> BTreeSet<String> {
    let mut options = BTreeSet::new();
    collect_command_options(command, &mut options);
    options
}

fn collect_command_options(command: &Command, options: &mut BTreeSet<String>) {
    options.extend(
        command
            .get_arguments()
            .filter_map(clap::Arg::get_long)
            .map(|name| format!("--{name}")),
    );
    for subcommand in command.get_subcommands() {
        collect_command_options(subcommand, options);
    }
}

fn configuration_keys(documents: &[ScannedDocument]) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for document in documents.iter().filter(|document| {
        matches!(
            document.category,
            FileCategory::Configuration | FileCategory::ProjectMetadata
        )
    }) {
        match document
            .relative_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("toml") => {
                if let Ok(value) = toml::from_str::<toml::Value>(&document.content) {
                    collect_toml_keys(&value, &mut keys);
                }
            }
            Some("json") => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&document.content) {
                    collect_json_keys(&value, &mut keys);
                }
            }
            _ => {}
        }
    }
    keys
}

fn collect_toml_keys(value: &toml::Value, keys: &mut BTreeSet<String>) {
    if let toml::Value::Table(table) = value {
        for (key, value) in table {
            keys.insert(key.clone());
            collect_toml_keys(value, keys);
        }
    } else if let toml::Value::Array(values) = value {
        for value in values {
            collect_toml_keys(value, keys);
        }
    }
}

fn collect_json_keys(value: &serde_json::Value, keys: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                keys.insert(key.clone());
                collect_json_keys(value, keys);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_json_keys(value, keys);
            }
        }
        _ => {}
    }
}

fn path_reference(value: &str) -> Option<String> {
    let value = trim_token(value).split('#').next().unwrap_or_default();
    if value.is_empty()
        || value.contains("://")
        || value.starts_with('#')
        || value.contains(char::is_whitespace)
    {
        return None;
    }
    let path = Path::new(value);
    let has_separator = value.contains('/') || value.contains('\\');
    let has_extension = path.extension().is_some();
    let normalized = normalize_reference_path(value);
    let generated = ["target/", "build/", "dist/", "node_modules/"]
        .iter()
        .any(|prefix| normalized.starts_with(prefix));
    (has_separator && has_extension && !generated).then(|| value.to_owned())
}

fn shun_invocation_start(words: &[&str]) -> Option<usize> {
    if let Some(executable) = words.first() {
        let executable = executable
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(executable.as_str(), "shun" | "shun.exe") {
            return Some(1);
        }
    }
    if words.starts_with(&["cargo", "run"]) {
        return words
            .iter()
            .rposition(|word| *word == "--")
            .map(|index| index + 1);
    }
    None
}

fn should_ignore_line(content: &str, line: usize) -> bool {
    content
        .lines()
        .nth(line.saturating_sub(1))
        .map(str::to_ascii_lowercase)
        .is_some_and(|line| {
            ["planned", "historical", "deprecated example", "deferred"]
                .iter()
                .any(|marker| line.contains(marker))
        })
}

fn normalize_reference_path(value: &str) -> String {
    normalize_repository_path(Path::new(value))
}

fn line_for_offset(content: &str, offset: usize) -> usize {
    let mut offset = offset.min(content.len());
    while offset > 0 && !content.is_char_boundary(offset) {
        offset -= 1;
    }
    content[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn trim_token(value: &str) -> &str {
    value.trim_matches(|character: char| {
        matches!(
            character,
            '`' | '"' | '\'' | ',' | ';' | ':' | '.' | '(' | ')' | '[' | ']' | '{' | '}'
        )
    })
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_alphabetic() || character == '_')
        && value.chars().all(is_identifier_character)
}

fn is_identifier_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn is_option_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '-' | '_')
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use clap::CommandFactory;

    use crate::cli::Cli;
    use crate::tokenizer::tokenize;

    use super::*;

    fn document(path: &str, category: FileCategory, content: &str) -> ScannedDocument {
        let tokenized = tokenize(content);
        ScannedDocument {
            absolute_path: Path::new("/").join(path),
            relative_path: PathBuf::from(path),
            category,
            content: content.to_owned(),
            token_count: tokenized.source_token_count,
            tokens: tokenized.tokens,
        }
    }

    #[test]
    fn reports_only_unresolved_code_like_markdown_references() {
        let documents = vec![
            document(
                "README.md",
                FileCategory::Documentation,
                "Use [`src/index.rs`](src/index.rs) and `SearchIndex`.\n\nMissing: `src/old.rs`, `initialize_server()`, `shun update`, `--threads`, and `legacy_path = true`.\n",
            ),
            document(
                "src/index.rs",
                FileCategory::SourceCode,
                "pub struct SearchIndex;",
            ),
            document(
                "config/app.toml",
                FileCategory::Configuration,
                "storage_path = \"index\"",
            ),
        ];
        let index = SearchIndex::build(&documents);
        let mut command = Cli::command();
        command.build();

        let audit = DocumentationAudit::build(&documents, &index, &command);

        assert_eq!(audit.documents_checked, 1);
        assert_eq!(audit.references_checked, 7);
        assert_eq!(audit.findings.len(), 5);
        assert!(audit.findings.iter().any(|finding| {
            finding.reference.kind == ReferenceKind::Path
                && finding.reference.value == "src/old.rs"
                && finding.confidence == Confidence::High
        }));
        assert!(audit.findings.iter().any(|finding| {
            finding.reference.kind == ReferenceKind::Symbol
                && finding.reference.value == "initialize_server"
                && finding.confidence == Confidence::Medium
        }));
        assert!(audit.findings.iter().any(|finding| {
            finding.reference.kind == ReferenceKind::Command
                && finding.reference.value == "update"
                && finding.confidence == Confidence::High
        }));
        assert!(audit.findings.iter().any(|finding| {
            finding.reference.kind == ReferenceKind::Option
                && finding.reference.value == "--threads"
                && finding.confidence == Confidence::High
        }));
        assert!(audit.findings.iter().any(|finding| {
            finding.reference.kind == ReferenceKind::ConfigurationKey
                && finding.reference.value == "legacy_path"
                && finding.confidence == Confidence::Low
        }));
    }

    #[test]
    fn resolves_parent_relative_links_and_path_invocations() {
        let documents = vec![
            document(
                "docs/guide.md",
                FileCategory::Documentation,
                "See [`index`](../src/index.rs). Run `./shun overview`.",
            ),
            document(
                "src/index.rs",
                FileCategory::SourceCode,
                "pub struct Index;",
            ),
        ];
        let index = SearchIndex::build(&documents);
        let mut command = Cli::command();
        command.build();

        let audit = DocumentationAudit::build(&documents, &index, &command);

        assert!(audit.findings.is_empty());
        assert_eq!(line_for_offset("é\ncode", 1), 1);
    }
}
