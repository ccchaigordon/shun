/* ============================================================================
 * Shun: symbol.rs
 * ============================================================================
 *
 * This file parses Rust documents with syn and builds a line-aware symbol
 * index. Symbol references are intentionally reported as likely text matches,
 * not compiler-resolved references.
 *
 * ============================================================================
 */

use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::index::DocumentId;
use crate::scanner::ScannedDocument;
use crate::tokenizer::tokenize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Implementation,
    Module,
    Constant,
    Static,
    TypeAlias,
    Macro,
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Function => "Function",
            Self::Struct => "Struct",
            Self::Enum => "Enum",
            Self::Trait => "Trait",
            Self::Implementation => "Implementation",
            Self::Module => "Module",
            Self::Constant => "Constant",
            Self::Static => "Static",
            Self::TypeAlias => "Type alias",
            Self::Macro => "Macro",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SymbolVisibility {
    Public,
    Crate,
    Restricted,
    Private,
}

impl fmt::Display for SymbolVisibility {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Public => "public",
            Self::Crate => "crate",
            Self::Restricted => "restricted",
            Self::Private => "private",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Symbol {
    pub(crate) name: String,
    pub(crate) kind: SymbolKind,
    pub(crate) document_id: DocumentId,
    pub(crate) line: usize,
    pub(crate) visibility: SymbolVisibility,
    pub(crate) parent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SymbolParseFailure {
    pub(crate) path: PathBuf,
    pub(crate) message: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct SymbolIndex {
    pub(crate) symbols: Vec<Symbol>,
    pub(crate) parse_failures: Vec<SymbolParseFailure>,
}

impl SymbolIndex {
    pub(crate) fn build(documents: &[ScannedDocument]) -> Self {
        let mut index = Self::default();

        for (document_id, document) in documents.iter().enumerate() {
            if !document
                .relative_path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
            {
                continue;
            }

            match syn::parse_file(&document.content) {
                Ok(file) => {
                    let mut collector = SymbolCollector::new(document_id);
                    collector.visit_file(&file);
                    index.symbols.extend(collector.symbols);
                }
                Err(error) => index.parse_failures.push(SymbolParseFailure {
                    path: document.relative_path.clone(),
                    message: error.to_string(),
                }),
            }
        }

        index.symbols.sort_by(|left, right| {
            left.document_id
                .cmp(&right.document_id)
                .then_with(|| left.line.cmp(&right.line))
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.name.cmp(&right.name))
        });
        index
    }

    pub(crate) fn lookup(&self, documents: &[ScannedDocument], query: &str) -> SymbolLookup {
        let definitions = self
            .symbols
            .iter()
            .filter(|symbol| symbol.name == query)
            .cloned()
            .collect::<Vec<_>>();
        if definitions.is_empty() {
            return SymbolLookup {
                query: query.to_owned(),
                definitions,
                references: Vec::new(),
            };
        }

        let normalized_query = tokenize(query)
            .tokens
            .into_iter()
            .next()
            .map(|token| token.term)
            .unwrap_or_default();
        let definition_locations = definitions
            .iter()
            .map(|symbol| (symbol.document_id, symbol.line))
            .collect::<BTreeSet<_>>();
        let mut reference_locations = BTreeSet::new();

        for (document_id, document) in documents.iter().enumerate() {
            for token in &document.tokens {
                if token.term == normalized_query
                    && !definition_locations.contains(&(document_id, token.line))
                {
                    reference_locations.insert((document_id, token.line));
                }
            }
        }

        SymbolLookup {
            query: query.to_owned(),
            definitions,
            references: reference_locations
                .into_iter()
                .map(|(document_id, line)| SymbolReference { document_id, line })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SymbolReference {
    pub(crate) document_id: DocumentId,
    pub(crate) line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SymbolLookup {
    pub(crate) query: String,
    pub(crate) definitions: Vec<Symbol>,
    pub(crate) references: Vec<SymbolReference>,
}

struct SymbolCollector {
    document_id: DocumentId,
    symbols: Vec<Symbol>,
    parents: Vec<String>,
}

impl SymbolCollector {
    fn new(document_id: DocumentId) -> Self {
        Self {
            document_id,
            symbols: Vec::new(),
            parents: Vec::new(),
        }
    }

    fn push(&mut self, name: String, kind: SymbolKind, visibility: &syn::Visibility, span: Span) {
        self.symbols.push(Symbol {
            name,
            kind,
            document_id: self.document_id,
            line: span.start().line,
            visibility: symbol_visibility(visibility),
            parent: self.parents.last().cloned(),
        });
    }

    fn with_parent(&mut self, parent: String, visit: impl FnOnce(&mut Self)) {
        self.parents.push(parent);
        visit(self);
        self.parents.pop();
    }
}

impl<'ast> Visit<'ast> for SymbolCollector {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.push(
            item.sig.ident.to_string(),
            SymbolKind::Function,
            &item.vis,
            item.sig.ident.span(),
        );
        visit::visit_item_fn(self, item);
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.push(
            item.ident.to_string(),
            SymbolKind::Struct,
            &item.vis,
            item.ident.span(),
        );
        visit::visit_item_struct(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.push(
            item.ident.to_string(),
            SymbolKind::Enum,
            &item.vis,
            item.ident.span(),
        );
        visit::visit_item_enum(self, item);
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        let name = item.ident.to_string();
        self.push(
            name.clone(),
            SymbolKind::Trait,
            &item.vis,
            item.ident.span(),
        );
        self.with_parent(name, |collector| visit::visit_item_trait(collector, item));
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.push(
            item.sig.ident.to_string(),
            SymbolKind::Function,
            &syn::Visibility::Inherited,
            item.sig.ident.span(),
        );
        visit::visit_trait_item_fn(self, item);
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let name = type_name(&item.self_ty).unwrap_or_else(|| "<anonymous>".to_owned());
        self.push(
            name.clone(),
            SymbolKind::Implementation,
            &syn::Visibility::Inherited,
            item.impl_token.span(),
        );
        self.with_parent(name, |collector| visit::visit_item_impl(collector, item));
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.push(
            item.sig.ident.to_string(),
            SymbolKind::Function,
            &item.vis,
            item.sig.ident.span(),
        );
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let name = item.ident.to_string();
        self.push(
            name.clone(),
            SymbolKind::Module,
            &item.vis,
            item.ident.span(),
        );
        self.with_parent(name, |collector| visit::visit_item_mod(collector, item));
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.push(
            item.ident.to_string(),
            SymbolKind::Constant,
            &item.vis,
            item.ident.span(),
        );
        visit::visit_item_const(self, item);
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.push(
            item.ident.to_string(),
            SymbolKind::Static,
            &item.vis,
            item.ident.span(),
        );
        visit::visit_item_static(self, item);
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.push(
            item.ident.to_string(),
            SymbolKind::TypeAlias,
            &item.vis,
            item.ident.span(),
        );
        visit::visit_item_type(self, item);
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if let Some(ident) = &item.ident {
            self.push(
                ident.to_string(),
                SymbolKind::Macro,
                &syn::Visibility::Inherited,
                ident.span(),
            );
        }
        visit::visit_item_macro(self, item);
    }
}

fn type_name(item_type: &syn::Type) -> Option<String> {
    let syn::Type::Path(type_path) = item_type else {
        return None;
    };
    type_path
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn symbol_visibility(visibility: &syn::Visibility) -> SymbolVisibility {
    match visibility {
        syn::Visibility::Public(_) => SymbolVisibility::Public,
        syn::Visibility::Restricted(restricted) if restricted.path.is_ident("crate") => {
            SymbolVisibility::Crate
        }
        syn::Visibility::Restricted(_) => SymbolVisibility::Restricted,
        syn::Visibility::Inherited => SymbolVisibility::Private,
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::classifier::FileCategory;
    use crate::tokenizer::tokenize;

    use super::*;

    fn rust_document(content: &str) -> ScannedDocument {
        let tokenized = tokenize(content);
        ScannedDocument {
            absolute_path: Path::new("/").join("src/lib.rs"),
            relative_path: PathBuf::from("src/lib.rs"),
            category: FileCategory::SourceCode,
            content: content.to_owned(),
            token_count: tokenized.source_token_count,
            tokens: tokenized.tokens,
        }
    }

    #[test]
    fn extracts_rust_symbols_with_parents_visibility_and_lines() {
        let document = rust_document(
            "pub struct Engine;\n\
             enum State { Ready }\n\
             pub trait Run { fn run(&self); }\n\
             impl Run for Engine { pub fn start(&self) {} }\n\
             mod nested { pub(crate) const LIMIT: usize = 1; static ENABLED: bool = true; }\n\
             type Id = usize;\n\
             macro_rules! make_engine { () => {} }\n\
             pub fn build() {}",
        );

        let index = SymbolIndex::build(&[document]);

        assert!(index.parse_failures.is_empty());
        assert!(index.symbols.iter().any(|symbol| {
            symbol.name == "Engine"
                && symbol.kind == SymbolKind::Struct
                && symbol.visibility == SymbolVisibility::Public
                && symbol.line == 1
        }));
        assert!(index.symbols.iter().any(|symbol| {
            symbol.name == "run"
                && symbol.kind == SymbolKind::Function
                && symbol.parent.as_deref() == Some("Run")
        }));
        assert!(index.symbols.iter().any(|symbol| {
            symbol.name == "start"
                && symbol.kind == SymbolKind::Function
                && symbol.parent.as_deref() == Some("Engine")
        }));
        assert!(index.symbols.iter().any(|symbol| {
            symbol.name == "LIMIT"
                && symbol.kind == SymbolKind::Constant
                && symbol.visibility == SymbolVisibility::Crate
                && symbol.parent.as_deref() == Some("nested")
        }));
        assert!(
            index
                .symbols
                .iter()
                .any(|symbol| symbol.name == "ENABLED" && symbol.kind == SymbolKind::Static)
        );
        assert!(
            index
                .symbols
                .iter()
                .any(|symbol| symbol.name == "Id" && symbol.kind == SymbolKind::TypeAlias)
        );
        assert!(
            index
                .symbols
                .iter()
                .any(|symbol| { symbol.name == "make_engine" && symbol.kind == SymbolKind::Macro })
        );
        assert!(
            index
                .symbols
                .iter()
                .any(|symbol| symbol.name == "build" && symbol.line == 8)
        );
    }

    #[test]
    fn finds_exact_definitions_and_likely_text_references() {
        let mut reference_document = rust_document("SearchIndex::build();\nSearchIndex::search();");
        reference_document.absolute_path = Path::new("/").join("src/main.rs");
        reference_document.relative_path = PathBuf::from("src/main.rs");
        let documents = vec![
            rust_document("pub struct SearchIndex;\nimpl SearchIndex {}"),
            reference_document,
        ];
        let index = SymbolIndex::build(&documents);

        let lookup = index.lookup(&documents, "SearchIndex");

        assert_eq!(lookup.definitions.len(), 2);
        assert_eq!(lookup.definitions[0].kind, SymbolKind::Struct);
        assert_eq!(lookup.definitions[1].kind, SymbolKind::Implementation);
        assert_eq!(
            lookup.references,
            [
                SymbolReference {
                    document_id: 1,
                    line: 1,
                },
                SymbolReference {
                    document_id: 1,
                    line: 2,
                },
            ]
        );
        assert!(
            index
                .lookup(&documents, "searchindex")
                .definitions
                .is_empty()
        );
    }
}
