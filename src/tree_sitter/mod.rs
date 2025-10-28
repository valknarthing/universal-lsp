//! Tree-sitter Integration
//!
//! This module provides tree-sitter parsing capabilities for extracting symbols,
//! definitions, and references from source code.

use anyhow::{Result, Context};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tree_sitter::{Language, Parser, Tree};

/// Global language registry
static LANGUAGE_REGISTRY: Lazy<DashMap<String, Language>> = Lazy::new(|| {
    let registry = DashMap::new();

    // Web & JavaScript ecosystem
    registry.insert("javascript".to_string(), tree_sitter_javascript::language());
    registry.insert("typescript".to_string(), tree_sitter_typescript::language_typescript());
    registry.insert("tsx".to_string(), tree_sitter_typescript::language_tsx());

    // Web core languages
    registry.insert("html".to_string(), tree_sitter_html::language());
    registry.insert("css".to_string(), tree_sitter_css::language());
    registry.insert("json".to_string(), tree_sitter_json::language());
    registry.insert("svelte".to_string(), tree_sitter_svelte::language());

    // Systems languages
    registry.insert("python".to_string(), tree_sitter_python::language());
    registry.insert("rust".to_string(), tree_sitter_rust::language());
    registry.insert("go".to_string(), tree_sitter_go::language());
    registry.insert("java".to_string(), tree_sitter_java::language());
    registry.insert("c".to_string(), tree_sitter_c::language());
    registry.insert("cpp".to_string(), tree_sitter_cpp::language());

    // Shell & scripts
    registry.insert("bash".to_string(), tree_sitter_bash::language());
    registry.insert("sh".to_string(), tree_sitter_bash::language());  // Alias for bash

    // Data & documentation formats
    // registry.insert("markdown".to_string(), tree_sitter_markdown::language());  // Disabled: uses tree-sitter 0.19.5
    // registry.insert("yaml".to_string(), tree_sitter_yaml::language());  // REMOVED: YAML requires tree-sitter 0.21
    // registry.insert("yml".to_string(), tree_sitter_yaml::language());  // Alias for yaml

    // Database
    // registry.insert("sql".to_string(), tree_sitter_sequel::language());  // REMOVED: SQL requires tree-sitter 0.21

    registry
});

/// Symbol information extracted from tree-sitter
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub selection_range: Range,
    pub detail: Option<String>,
}

/// Definition information
#[derive(Debug, Clone)]
pub struct Definition {
    pub name: String,
    pub range: Range,
    pub uri: Url,
}

/// Reference information
#[derive(Debug, Clone)]
pub struct Reference {
    pub range: Range,
    pub uri: Url,
}

/// Tree-sitter parser with caching
pub struct TreeSitterParser {
    parser: Parser,
    language: Option<Language>,
    tree_cache: DashMap<String, Arc<Tree>>,
}

impl std::fmt::Debug for TreeSitterParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeSitterParser")
            .field("language", &self.language.is_some())
            .field("cached_trees", &self.tree_cache.len())
            .finish()
    }
}

impl TreeSitterParser {
    /// Create a new parser
    pub fn new() -> Result<Self> {
        Ok(Self {
            parser: Parser::new(),
            language: None,
            tree_cache: DashMap::new(),
        })
    }

    /// Set language for parsing
    pub fn set_language(&mut self, lang: &str) -> Result<()> {
        if let Some(language) = LANGUAGE_REGISTRY.get(lang) {
            self.parser.set_language(*language)?;
            self.language = Some(language.clone());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Language {} not supported", lang))
        }
    }

    /// Parse source code
    pub fn parse(&mut self, source: &str, uri: &str) -> Result<Arc<Tree>> {
        let tree = self.parser.parse(source, None)
            .context("Failed to parse source code")?;

        let arc_tree = Arc::new(tree);
        self.tree_cache.insert(uri.to_string(), arc_tree.clone());
        Ok(arc_tree)
    }

    /// Get cached tree
    pub fn get_cached_tree(&self, uri: &str) -> Option<Arc<Tree>> {
        self.tree_cache.get(uri).map(|t| t.clone())
    }

    /// Extract symbols from tree
    pub fn extract_symbols(&self, tree: &Tree, source: &str, lang: &str) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let root_node = tree.root_node();

        // Language-specific symbol extraction
        match lang {
            "javascript" | "typescript" | "tsx" => {
                self.extract_js_symbols(root_node, source, &mut symbols)?;
            }
            "python" => {
                self.extract_python_symbols(root_node, source, &mut symbols)?;
            }
            "rust" => {
                self.extract_rust_symbols(root_node, source, &mut symbols)?;
            }
            "go" => {
                self.extract_go_symbols(root_node, source, &mut symbols)?;
            }
            "java" => {
                self.extract_java_symbols(root_node, source, &mut symbols)?;
            }
            "c" | "cpp" => {
                self.extract_c_symbols(root_node, source, &mut symbols)?;
            }
            "svelte" => {
                self.extract_svelte_symbols(root_node, source, &mut symbols)?;
            }
            "bash" | "sh" => {
                self.extract_bash_symbols(root_node, source, &mut symbols)?;
            }
            "css" => {
                self.extract_css_symbols(root_node, source, &mut symbols)?;
            }
            "html" => {
                self.extract_html_symbols(root_node, source, &mut symbols)?;
            }
            "json" => {
                self.extract_json_symbols(root_node, source, &mut symbols)?;
            }
            // "markdown" => {
            //     self.extract_markdown_symbols(root_node, source, &mut symbols)?;
            // }
            // SQL disabled: requires tree-sitter 0.21
            // "sql" => {
            //     self.extract_sql_symbols(root_node, source, &mut symbols)?;
            // }
            // YAML disabled: requires tree-sitter 0.21
            // "yaml" | "yml" => {
            //     self.extract_yaml_symbols(root_node, source, &mut symbols)?;
            // }
            _ => {}
        }

        Ok(symbols)
    }

    /// Find definition at position
    pub fn find_definition(
        &self,
        tree: &Tree,
        source: &str,
        position: Position,
        lang: &str
    ) -> Result<Option<Definition>> {
        let byte_offset = self.position_to_byte(source, position);
        let Some(node) = tree.root_node().descendant_for_byte_range(byte_offset, byte_offset) else {
            return Ok(None);
        };

        // Check if we're on an identifier
        if node.kind() == "identifier" || node.kind() == "type_identifier" {
            let name = &source[node.byte_range()];

            // Search for definition
            if let Some(def_node) = self.find_definition_node(tree.root_node(), name, lang) {
                let range = self.node_to_range(&def_node, source)?;
                return Ok(Some(Definition {
                    name: name.to_string(),
                    range,
                    uri: Url::parse("file:///temp")?, // Will be filled by caller
                }));
            }
        }

        Ok(None)
    }

    /// Find all references to symbol
    pub fn find_references(
        &self,
        tree: &Tree,
        source: &str,
        position: Position,
        _lang: &str
    ) -> Result<Vec<Reference>> {
        let byte_offset = self.position_to_byte(source, position);
        let Some(node) = tree.root_node().descendant_for_byte_range(byte_offset, byte_offset) else {
            return Ok(Vec::new());
        };

        let mut references = Vec::new();

        if node.kind() == "identifier" || node.kind() == "type_identifier" {
            let name = &source[node.byte_range()];
            self.find_identifier_references(tree.root_node(), name, source, &mut references)?;
        }

        Ok(references)
    }

    // === Helper methods ===

    fn position_to_byte(&self, source: &str, position: Position) -> usize {
        let mut byte_offset = 0;
        let mut current_line = 0;
        let mut current_char = 0;

        for ch in source.chars() {
            if current_line == position.line && current_char == position.character {
                break;
            }
            byte_offset += ch.len_utf8();
            current_char += 1;
            if ch == '\n' {
                current_line += 1;
                current_char = 0;
            }
        }

        byte_offset
    }

    fn node_to_range(&self, node: &tree_sitter::Node, source: &str) -> Result<Range> {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();

        let start = self.byte_to_position(source, start_byte)?;
        let end = self.byte_to_position(source, end_byte)?;

        Ok(Range { start, end })
    }

    fn byte_to_position(&self, source: &str, byte_offset: usize) -> Result<Position> {
        let mut line = 0;
        let mut character = 0;
        let mut current_byte = 0;

        for ch in source.chars() {
            if current_byte >= byte_offset {
                break;
            }
            current_byte += ch.len_utf8();
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }

        Ok(Position { line, character })
    }

    fn find_definition_node<'a>(
        &self,
        node: tree_sitter::Node<'a>,
        name: &str,
        lang: &str
    ) -> Option<tree_sitter::Node<'a>> {
        let definition_kinds = match lang {
            "javascript" | "typescript" | "tsx" => vec![
                "function_declaration",
                "class_declaration",
                "variable_declaration",
                "lexical_declaration",
            ],
            "python" => vec!["function_definition", "class_definition"],
            "rust" => vec!["function_item", "struct_item", "enum_item", "impl_item"],
            "go" => vec!["function_declaration", "type_declaration"],
            "java" => vec!["method_declaration", "class_declaration", "field_declaration"],
            "c" | "cpp" => vec!["function_definition", "declaration"],
            "svelte" => vec![
                "function_declaration",
                "const_declaration",
                "let_declaration",
                "script_element",
            ],
            _ => vec![],
        };

        self.search_node_recursive(node, |n| {
            if definition_kinds.contains(&n.kind()) {
                // Check if this node defines our identifier
                for child in n.children(&mut n.walk()) {
                    if (child.kind() == "identifier" || child.kind() == "type_identifier")
                        && child.utf8_text(&[]).ok() == Some(name) {
                        return true;
                    }
                }
            }
            false
        })
    }

    fn find_identifier_references(
        &self,
        node: tree_sitter::Node,
        name: &str,
        source: &str,
        references: &mut Vec<Reference>
    ) -> Result<()> {
        if node.kind() == "identifier" || node.kind() == "type_identifier" {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                if text == name {
                    let range = self.node_to_range(&node, source)?;
                    references.push(Reference {
                        range,
                        uri: Url::parse("file:///temp")?,
                    });
                }
            }
        }

        for child in node.children(&mut node.walk()) {
            self.find_identifier_references(child, name, source, references)?;
        }

        Ok(())
    }

    fn search_node_recursive<'a, F>(&self, node: tree_sitter::Node<'a>, predicate: F) -> Option<tree_sitter::Node<'a>>
    where
        F: Fn(&tree_sitter::Node) -> bool + Copy,
    {
        if predicate(&node) {
            return Some(node);
        }

        for child in node.children(&mut node.walk()) {
            if let Some(found) = self.search_node_recursive(child, predicate) {
                return Some(found);
            }
        }

        None
    }

    // === Language-specific symbol extraction ===

    fn extract_js_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "function_declaration" | "function_expression" | "arrow_function" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::FUNCTION,
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some("function".to_string()),
                    });
                }
            }
            "class_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::CLASS,
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some("class".to_string()),
                    });
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_js_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_python_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::FUNCTION,
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some("def".to_string()),
                    });
                }
            }
            "class_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::CLASS,
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some("class".to_string()),
                    });
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_python_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_rust_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "function_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::FUNCTION,
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some("fn".to_string()),
                    });
                }
            }
            "struct_item" | "enum_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: if node.kind() == "struct_item" {
                            SymbolKind::STRUCT
                        } else {
                            SymbolKind::ENUM
                        },
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some(node.kind().replace("_item", "").to_string()),
                    });
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_rust_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_go_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "function_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::FUNCTION,
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some("func".to_string()),
                    });
                }
            }
            "type_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::STRUCT,
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some("type".to_string()),
                    });
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_go_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_java_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "method_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::METHOD,
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some("method".to_string()),
                    });
                }
            }
            "class_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::CLASS,
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some("class".to_string()),
                    });
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_java_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_c_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                if let Some(declarator) = node.child_by_field_name("declarator") {
                    if let Some(name_node) = declarator.child_by_field_name("declarator") {
                        let name = name_node.utf8_text(source.as_bytes())?;
                        let range = self.node_to_range(&node, source)?;
                        symbols.push(Symbol {
                            name: name.to_string(),
                            kind: SymbolKind::FUNCTION,
                            range,
                            selection_range: self.node_to_range(&name_node, source)?,
                            detail: Some("function".to_string()),
                        });
                    }
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_c_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_svelte_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "function_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::FUNCTION,
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some("function".to_string()),
                    });
                }
            }
            "lexical_declaration" => {
                // Handle const/let declarations in script blocks
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "variable_declarator" {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            let name = name_node.utf8_text(source.as_bytes())?;
                            let range = self.node_to_range(&child, source)?;
                            symbols.push(Symbol {
                                name: name.to_string(),
                                kind: SymbolKind::VARIABLE,
                                range,
                                selection_range: self.node_to_range(&name_node, source)?,
                                detail: Some("variable".to_string()),
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_svelte_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_bash_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: name.to_string(),
                        kind: SymbolKind::FUNCTION,
                        range,
                        selection_range: self.node_to_range(&name_node, source)?,
                        detail: Some("function".to_string()),
                    });
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_bash_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_css_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "rule_set" | "class_selector" | "id_selector" => {
                // Extract CSS selectors
                if let Some(name_node) = node.child(0) {
                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                        let range = self.node_to_range(&node, source)?;
                        symbols.push(Symbol {
                            name: name.to_string(),
                            kind: SymbolKind::CLASS,
                            range,
                            selection_range: self.node_to_range(&name_node, source)?,
                            detail: Some("selector".to_string()),
                        });
                    }
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_css_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_html_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "element" | "self_closing_tag" => {
                if let Some(start_tag) = node.child_by_field_name("start_tag") {
                    if let Some(name_node) = start_tag.child_by_field_name("tag_name") {
                        let name = name_node.utf8_text(source.as_bytes())?;
                        // Look for id attribute
                        if let Some(id) = self.get_html_id_attribute(&node, source) {
                            let range = self.node_to_range(&node, source)?;
                            symbols.push(Symbol {
                                name: format!("{} (id={})", name, id),
                                kind: SymbolKind::STRUCT,
                                range,
                                selection_range: self.node_to_range(&name_node, source)?,
                                detail: Some("element".to_string()),
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_html_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn get_html_id_attribute(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "attribute" {
                if let Some(name) = child.child_by_field_name("name") {
                    if let Ok(attr_name) = name.utf8_text(source.as_bytes()) {
                        if attr_name == "id" {
                            if let Some(value) = child.child_by_field_name("value") {
                                if let Ok(id) = value.utf8_text(source.as_bytes()) {
                                    return Some(id.trim_matches('"').to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn extract_json_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "pair" => {
                if let Some(key_node) = node.child_by_field_name("key") {
                    let key = key_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: key.trim_matches('"').to_string(),
                        kind: SymbolKind::FIELD,
                        range,
                        selection_range: self.node_to_range(&key_node, source)?,
                        detail: Some("property".to_string()),
                    });
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_json_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_markdown_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "atx_heading" | "setext_heading" => {
                // Extract markdown headings
                if let Ok(heading_text) = node.utf8_text(source.as_bytes()) {
                    let heading_text = heading_text.trim();
                    let range = self.node_to_range(&node, source)?;

                    // Determine heading level
                    let level = if heading_text.starts_with("######") {
                        6
                    } else if heading_text.starts_with("#####") {
                        5
                    } else if heading_text.starts_with("####") {
                        4
                    } else if heading_text.starts_with("###") {
                        3
                    } else if heading_text.starts_with("##") {
                        2
                    } else {
                        1
                    };

                    let title = heading_text.trim_start_matches('#').trim();

                    symbols.push(Symbol {
                        name: title.to_string(),
                        kind: SymbolKind::STRING,  // Using STRING for headings
                        range,
                        selection_range: range,
                        detail: Some(format!("h{}", level)),
                    });
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_markdown_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_sql_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "create_table_statement" | "create_view_statement" | "create_function_statement" => {
                // Extract SQL object names
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "identifier" || child.kind() == "table_name" {
                        let name = child.utf8_text(source.as_bytes())?;
                        let range = self.node_to_range(&node, source)?;
                        symbols.push(Symbol {
                            name: name.to_string(),
                            kind: SymbolKind::CLASS,
                            range,
                            selection_range: self.node_to_range(&child, source)?,
                            detail: Some(node.kind().replace("_statement", "").to_string()),
                        });
                        break;
                    }
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_sql_symbols(child, source, symbols)?;
        }

        Ok(())
    }

    fn extract_yaml_symbols(
        &self,
        node: tree_sitter::Node,
        source: &str,
        symbols: &mut Vec<Symbol>
    ) -> Result<()> {
        match node.kind() {
            "block_mapping_pair" => {
                if let Some(key_node) = node.child_by_field_name("key") {
                    let key = key_node.utf8_text(source.as_bytes())?;
                    let range = self.node_to_range(&node, source)?;
                    symbols.push(Symbol {
                        name: key.to_string(),
                        kind: SymbolKind::FIELD,
                        range,
                        selection_range: self.node_to_range(&key_node, source)?,
                        detail: Some("key".to_string()),
                    });
                }
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.extract_yaml_symbols(child, source, symbols)?;
        }

        Ok(())
    }
}

impl Default for TreeSitterParser {
    fn default() -> Self {
        Self::new().expect("Failed to create tree-sitter parser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = TreeSitterParser::new();
        assert!(parser.is_ok());
    }

    #[test]
    fn test_set_language() {
        let mut parser = TreeSitterParser::new().unwrap();
        assert!(parser.set_language("javascript").is_ok());
        assert!(parser.set_language("python").is_ok());
        assert!(parser.set_language("rust").is_ok());
    }

    #[test]
    fn test_parse_javascript() {
        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("javascript").unwrap();

        let source = "function hello() { return 'world'; }";
        let result = parser.parse(source, "test.js");
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_js_symbols() {
        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("javascript").unwrap();

        let source = "function hello() {}\nclass World {}";
        let tree = parser.parse(source, "test.js").unwrap();
        let symbols = parser.extract_symbols(&tree, source, "javascript").unwrap();

        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[1].name, "World");
    }

    #[test]
    fn test_extract_python_symbols() {
        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("python").unwrap();

        let source = "def hello():\n    pass\n\nclass World:\n    pass";
        let tree = parser.parse(source, "test.py").unwrap();
        let symbols = parser.extract_symbols(&tree, source, "python").unwrap();

        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[1].name, "World");
    }
}
