//! Code Actions and Refactoring Module
//!
//! Provides quick fixes, refactorings, and code transformations

use anyhow::Result;
use tower_lsp::lsp_types::*;
use crate::tree_sitter::TreeSitterParser;

/// Code action provider for refactoring and quick fixes
#[derive(Debug)]
pub struct CodeActionProvider {
    // Future: Add refactoring configurations
}

impl CodeActionProvider {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate code actions for a given range
    pub fn get_actions(
        &self,
        uri: &Url,
        range: Range,
        content: &str,
        diagnostics: Vec<Diagnostic>,
        lang: &str,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        // Add quick fixes for diagnostics
        for diagnostic in diagnostics {
            if let Some(action) = self.diagnostic_to_quick_fix(&diagnostic, uri, lang) {
                actions.push(action);
            }
        }

        // Add refactoring actions
        let mut parser = TreeSitterParser::new()?;
        if parser.set_language(lang).is_ok() {
            if let Ok(tree) = parser.parse(content, uri.as_str()) {
                actions.extend(self.get_refactoring_actions(&tree, content, range, uri, lang)?);
            }
        }

        Ok(actions)
    }

    /// Convert diagnostic to quick fix action
    fn diagnostic_to_quick_fix(
        &self,
        diagnostic: &Diagnostic,
        uri: &Url,
        lang: &str,
    ) -> Option<CodeActionOrCommand> {
        // Handle syntax errors
        if diagnostic.source.as_deref() == Some("tree-sitter") {
            return self.create_syntax_error_fix(diagnostic, uri, lang);
        }

        None
    }

    /// Create quick fix for syntax errors
    fn create_syntax_error_fix(
        &self,
        diagnostic: &Diagnostic,
        uri: &Url,
        _lang: &str,
    ) -> Option<CodeActionOrCommand> {
        // Example: Missing semicolon fix
        if diagnostic.message.contains("Missing") {
            let edit = TextEdit {
                range: diagnostic.range,
                new_text: diagnostic.message.replace("Missing ", ""),
            };

            let mut changes = std::collections::HashMap::new();
            changes.insert(uri.clone(), vec![edit]);

            return Some(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Add {}", diagnostic.message),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                }),
                ..Default::default()
            }));
        }

        None
    }

    /// Get refactoring actions for a range
    fn get_refactoring_actions(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        range: Range,
        uri: &Url,
        lang: &str,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        // Find node at range
        let start_byte = self.position_to_byte(source, range.start);
        let end_byte = self.position_to_byte(source, range.end);

        if let Some(node) = tree.root_node().descendant_for_byte_range(start_byte, end_byte) {
            // Language-specific refactorings
            match lang {
                "javascript" | "typescript" | "tsx" => {
                    actions.extend(self.js_ts_refactorings(node, source, uri)?);
                }
                "python" => {
                    actions.extend(self.python_refactorings(node, source, uri)?);
                }
                "rust" => {
                    actions.extend(self.rust_refactorings(node, source, uri)?);
                }
                _ => {}
            }

            // Generic refactorings
            actions.extend(self.generic_refactorings(node, source, uri)?);
        }

        Ok(actions)
    }

    /// JavaScript/TypeScript refactorings
    fn js_ts_refactorings(
        &self,
        node: tree_sitter::Node,
        _source: &str,
        _uri: &Url,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        match node.kind() {
            "function_declaration" => {
                // Example: Convert to arrow function
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Convert to arrow function".to_string(),
                    kind: Some(CodeActionKind::REFACTOR),
                    ..Default::default()
                }));
            }
            "variable_declaration" => {
                // Example: Convert var to const/let
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Convert var to const".to_string(),
                    kind: Some(CodeActionKind::REFACTOR),
                    ..Default::default()
                }));
            }
            _ => {}
        }

        Ok(actions)
    }

    /// Python refactorings
    fn python_refactorings(
        &self,
        node: tree_sitter::Node,
        _source: &str,
        _uri: &Url,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        match node.kind() {
            "function_definition" => {
                // Example: Extract to method
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Extract to method".to_string(),
                    kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                    ..Default::default()
                }));
            }
            "class_definition" => {
                // Example: Add docstring
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Add docstring".to_string(),
                    kind: Some(CodeActionKind::SOURCE),
                    ..Default::default()
                }));
            }
            _ => {}
        }

        Ok(actions)
    }

    /// Rust refactorings
    fn rust_refactorings(
        &self,
        node: tree_sitter::Node,
        _source: &str,
        _uri: &Url,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        match node.kind() {
            "function_item" => {
                // Example: Add error handling
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Add error handling (Result)".to_string(),
                    kind: Some(CodeActionKind::REFACTOR),
                    ..Default::default()
                }));
            }
            "impl_item" => {
                // Example: Derive common traits
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Derive Debug, Clone".to_string(),
                    kind: Some(CodeActionKind::REFACTOR),
                    ..Default::default()
                }));
            }
            _ => {}
        }

        Ok(actions)
    }

    /// Generic refactorings applicable to all languages
    fn generic_refactorings(
        &self,
        _node: tree_sitter::Node,
        _source: &str,
        _uri: &Url,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        // Example: Rename symbol
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Rename symbol".to_string(),
            kind: Some(CodeActionKind::REFACTOR_REWRITE),
            ..Default::default()
        }));

        // Example: Extract variable
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Extract variable".to_string(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            ..Default::default()
        }));

        Ok(actions)
    }

    /// Helper: Convert LSP position to byte offset
    fn position_to_byte(&self, source: &str, position: Position) -> usize {
        let mut byte_offset = 0;
        let mut current_line = 0;
        let mut current_char = 0;

        for ch in source.chars() {
            if current_line == position.line && current_char == position.character {
                return byte_offset;
            }

            if ch == '\n' {
                current_line += 1;
                current_char = 0;
            } else {
                current_char += 1;
            }

            byte_offset += ch.len_utf8();
        }

        byte_offset
    }
}

impl Default for CodeActionProvider {
    fn default() -> Self {
        Self::new()
    }
}
