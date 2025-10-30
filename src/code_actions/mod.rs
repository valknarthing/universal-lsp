//! Code Actions and Refactoring Module
//!
//! Provides quick fixes, refactorings, and code transformations

use anyhow::Result;
use tower_lsp::lsp_types::*;
use crate::tree_sitter::TreeSitterParser;
use crate::ai::claude::ClaudeClient;
use std::sync::Arc;

/// Code action provider for refactoring and quick fixes
#[derive(Debug)]
pub struct CodeActionProvider {
    claude_client: Option<Arc<ClaudeClient>>,
}

impl CodeActionProvider {
    pub fn new() -> Self {
        Self {
            claude_client: None,
        }
    }

    pub fn with_claude(claude_client: Option<Arc<ClaudeClient>>) -> Self {
        Self { claude_client }
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
            if let Some(action) = self.diagnostic_to_quick_fix(&diagnostic, uri, lang, content) {
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

        // Add AI-powered actions if Claude is available
        if self.claude_client.is_some() {
            actions.extend(self.get_ai_actions(content, range, uri, lang)?);
        }

        Ok(actions)
    }

    /// Convert diagnostic to quick fix action
    fn diagnostic_to_quick_fix(
        &self,
        diagnostic: &Diagnostic,
        uri: &Url,
        lang: &str,
        content: &str,
    ) -> Option<CodeActionOrCommand> {
        // Handle "Undefined name" warnings
        if diagnostic.message.starts_with("Undefined name '") {
            return self.create_undefined_name_fix(diagnostic, uri, lang, content);
        }

        // Handle syntax errors
        if diagnostic.source.as_deref() == Some("tree-sitter") {
            return self.create_syntax_error_fix(diagnostic, uri, lang);
        }

        None
    }

    /// Create quick fix for undefined name warnings
    fn create_undefined_name_fix(
        &self,
        diagnostic: &Diagnostic,
        uri: &Url,
        lang: &str,
        content: &str,
    ) -> Option<CodeActionOrCommand> {
        // Extract variable name from message: "Undefined name 'xxx'"
        let var_name = diagnostic.message
            .strip_prefix("Undefined name '")?
            .strip_suffix('\'')?;

        let mut actions = Vec::new();

        // Get line content to determine context
        let line_start = Position {
            line: diagnostic.range.start.line,
            character: 0,
        };
        let line_end = Position {
            line: diagnostic.range.start.line + 1,
            character: 0,
        };
        let line_start_byte = self.position_to_byte(content, line_start);
        let line_end_byte = self.position_to_byte(content, line_end).min(content.len());
        let line_content = &content[line_start_byte..line_end_byte];

        match lang {
            "python" => {
                // Python-specific quick fixes
                if line_content.contains(&format!("{}(", var_name)) {
                    // Looks like a function call - suggest defining a function
                    let insert_pos = Position { line: 0, character: 0 };
                    let new_text = format!("def {}():\n    pass\n\n", var_name);

                    let mut changes = std::collections::HashMap::new();
                    changes.insert(uri.clone(), vec![TextEdit {
                        range: Range {
                            start: insert_pos,
                            end: insert_pos,
                        },
                        new_text,
                    }]);

                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: format!("Define function '{}'", var_name),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diagnostic.clone()]),
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));
                } else {
                    // Looks like a variable - suggest defining it
                    let insert_pos = Position { line: diagnostic.range.start.line, character: 0 };
                    let indent = line_content.len() - line_content.trim_start().len();
                    let new_text = format!("{}{} = None  # TODO: Define this variable\n", " ".repeat(indent), var_name);

                    let mut changes = std::collections::HashMap::new();
                    changes.insert(uri.clone(), vec![TextEdit {
                        range: Range {
                            start: insert_pos,
                            end: insert_pos,
                        },
                        new_text,
                    }]);

                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: format!("Define variable '{}' before use", var_name),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diagnostic.clone()]),
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));
                }
            }
            "javascript" | "typescript" | "tsx" => {
                // JS/TS-specific quick fixes
                if line_content.contains(&format!("{}(", var_name)) {
                    // Suggest defining a function
                    let insert_pos = Position { line: 0, character: 0 };
                    let new_text = format!("function {}() {{\n  // TODO: Implement\n}}\n\n", var_name);

                    let mut changes = std::collections::HashMap::new();
                    changes.insert(uri.clone(), vec![TextEdit {
                        range: Range {
                            start: insert_pos,
                            end: insert_pos,
                        },
                        new_text,
                    }]);

                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: format!("Define function '{}'", var_name),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diagnostic.clone()]),
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));
                } else {
                    // Suggest defining a variable
                    let insert_pos = Position { line: diagnostic.range.start.line, character: 0 };
                    let indent = line_content.len() - line_content.trim_start().len();
                    let new_text = format!("{}const {} = undefined; // TODO: Define this variable\n", " ".repeat(indent), var_name);

                    let mut changes = std::collections::HashMap::new();
                    changes.insert(uri.clone(), vec![TextEdit {
                        range: Range {
                            start: insert_pos,
                            end: insert_pos,
                        },
                        new_text,
                    }]);

                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: format!("Define variable '{}'", var_name),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diagnostic.clone()]),
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));
                }
            }
            "rust" => {
                // Rust-specific quick fixes
                if line_content.contains(&format!("{}(", var_name)) || line_content.contains(&format!("{}!", var_name)) {
                    // Suggest defining a function
                    let insert_pos = Position { line: 0, character: 0 };
                    let new_text = format!("fn {}() {{\n    todo!()\n}}\n\n", var_name);

                    let mut changes = std::collections::HashMap::new();
                    changes.insert(uri.clone(), vec![TextEdit {
                        range: Range {
                            start: insert_pos,
                            end: insert_pos,
                        },
                        new_text,
                    }]);

                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: format!("Define function '{}'", var_name),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diagnostic.clone()]),
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));
                } else {
                    // Suggest defining a variable
                    let insert_pos = Position { line: diagnostic.range.start.line, character: 0 };
                    let indent = line_content.len() - line_content.trim_start().len();
                    let new_text = format!("{}let {} = todo!(); // TODO: Define this variable\n", " ".repeat(indent), var_name);

                    let mut changes = std::collections::HashMap::new();
                    changes.insert(uri.clone(), vec![TextEdit {
                        range: Range {
                            start: insert_pos,
                            end: insert_pos,
                        },
                        new_text,
                    }]);

                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: format!("Define variable '{}'", var_name),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diagnostic.clone()]),
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));
                }
            }
            _ => {}
        }

        // Return the first action (or None if no actions were created)
        actions.into_iter().next()
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

    /// Get AI-powered code actions
    fn get_ai_actions(
        &self,
        content: &str,
        range: Range,
        uri: &Url,
        _lang: &str,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        // Extract selected text
        let start_byte = self.position_to_byte(content, range.start);
        let end_byte = self.position_to_byte(content, range.end);
        let selected_text = if start_byte < end_byte && end_byte <= content.len() {
            &content[start_byte..end_byte]
        } else {
            ""
        };

        // Only show AI actions if there's selected text
        if !selected_text.trim().is_empty() {
            // "Explain code" action
            actions.push(CodeActionOrCommand::Command(Command {
                title: "🤖 Explain code with Claude".to_string(),
                command: "universal-lsp.explainCode".to_string(),
                arguments: Some(vec![
                    serde_json::to_value(uri.to_string()).unwrap(),
                    serde_json::to_value(range).unwrap(),
                ]),
            }));

            // "Optimize code" action
            actions.push(CodeActionOrCommand::Command(Command {
                title: "🤖 Optimize code with Claude".to_string(),
                command: "universal-lsp.optimizeCode".to_string(),
                arguments: Some(vec![
                    serde_json::to_value(uri.to_string()).unwrap(),
                    serde_json::to_value(range).unwrap(),
                ]),
            }));

            // "Add tests" action
            actions.push(CodeActionOrCommand::Command(Command {
                title: "🤖 Generate tests with Claude".to_string(),
                command: "universal-lsp.generateTests".to_string(),
                arguments: Some(vec![
                    serde_json::to_value(uri.to_string()).unwrap(),
                    serde_json::to_value(range).unwrap(),
                ]),
            }));

            // "Generate documentation" action
            actions.push(CodeActionOrCommand::Command(Command {
                title: "🤖 Generate documentation".to_string(),
                command: "universal-lsp.generateDocs".to_string(),
                arguments: Some(vec![
                    serde_json::to_value(uri.to_string()).unwrap(),
                    serde_json::to_value(range).unwrap(),
                ]),
            }));
        }

        Ok(actions)
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

        // Generic refactorings available when text is selected
        if start_byte < end_byte {
            actions.extend(self.generic_refactorings(tree, source, range, uri, lang)?);
        }

        if let Some(node) = tree.root_node().descendant_for_byte_range(start_byte, end_byte) {
            // Language-specific refactorings
            match lang {
                "javascript" | "typescript" | "tsx" => {
                    actions.extend(self.js_ts_refactorings(node, source, range, uri)?);
                }
                "python" => {
                    actions.extend(self.python_refactorings(node, source, range, uri)?);
                }
                "rust" => {
                    actions.extend(self.rust_refactorings(node, source, range, uri)?);
                }
                _ => {}
            }
        }

        Ok(actions)
    }

    /// JavaScript/TypeScript refactorings
    fn js_ts_refactorings(
        &self,
        node: tree_sitter::Node,
        source: &str,
        _range: Range,
        uri: &Url,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        match node.kind() {
            "function_declaration" => {
                // Suggest: Convert to arrow function (placeholder for now)
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Convert to arrow function".to_string(),
                    kind: Some(CodeActionKind::REFACTOR),
                    ..Default::default()
                }));
            }
            "variable_declaration" => {
                // Check if it uses 'var'
                let text = &source[node.start_byte()..node.end_byte()];
                if text.starts_with("var ") {
                    let new_text = text.replacen("var ", "const ", 1);
                    let mut changes = std::collections::HashMap::new();
                    changes.insert(uri.clone(), vec![TextEdit {
                        range: Range {
                            start: self.byte_to_position(source, node.start_byte()),
                            end: self.byte_to_position(source, node.end_byte()),
                        },
                        new_text,
                    }]);

                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: "Convert 'var' to 'const'".to_string(),
                        kind: Some(CodeActionKind::REFACTOR_REWRITE),
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }));
                }
            }
            _ => {}
        }

        Ok(actions)
    }

    /// Python refactorings
    fn python_refactorings(
        &self,
        node: tree_sitter::Node,
        source: &str,
        _range: Range,
        uri: &Url,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        match node.kind() {
            "function_definition" | "class_definition" => {
                // Check if there's a docstring
                let text = &source[node.start_byte()..node.end_byte()];
                if !text.contains("\"\"\"") && !text.contains("'''") {
                    // Find the position after the colon on the first line
                    if let Some(first_line_end) = text.find('\n') {
                        let insert_offset = node.start_byte() + first_line_end + 1;
                        let indent = "    "; // Basic indentation
                        let docstring = format!("{}\"\"\"TODO: Add description.\"\"\"\n", indent);

                        let mut changes = std::collections::HashMap::new();
                        changes.insert(uri.clone(), vec![TextEdit {
                            range: Range {
                                start: self.byte_to_position(source, insert_offset),
                                end: self.byte_to_position(source, insert_offset),
                            },
                            new_text: docstring,
                        }]);

                        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                            title: "Add docstring".to_string(),
                            kind: Some(CodeActionKind::SOURCE),
                            edit: Some(WorkspaceEdit {
                                changes: Some(changes),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }));
                    }
                }
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
        _range: Range,
        _uri: &Url,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        match node.kind() {
            "function_item" => {
                // Suggest: Add error handling (placeholder for now)
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Add error handling (Result<T, E>)".to_string(),
                    kind: Some(CodeActionKind::REFACTOR),
                    ..Default::default()
                }));
            }
            "struct_item" => {
                // Suggest: Derive common traits (placeholder for now)
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Add #[derive(Debug, Clone)]".to_string(),
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
        _tree: &tree_sitter::Tree,
        source: &str,
        range: Range,
        uri: &Url,
        lang: &str,
    ) -> Result<Vec<CodeActionOrCommand>> {
        let mut actions = Vec::new();

        let start_byte = self.position_to_byte(source, range.start);
        let end_byte = self.position_to_byte(source, range.end);

        // Only offer extract variable if there's a selection
        if start_byte < end_byte && end_byte <= source.len() {
            let selected_text = &source[start_byte..end_byte];

            // Extract variable refactoring
            if !selected_text.trim().is_empty() && !selected_text.contains('\n') {
                let variable_name = "extracted_value";

                let (declaration, assignment) = match lang {
                    "python" => (
                        format!("{} = {}\n", variable_name, selected_text.trim()),
                        variable_name.to_string(),
                    ),
                    "javascript" | "typescript" | "tsx" => (
                        format!("const {} = {};\n", variable_name, selected_text.trim()),
                        variable_name.to_string(),
                    ),
                    "rust" => (
                        format!("let {} = {};\n", variable_name, selected_text.trim()),
                        variable_name.to_string(),
                    ),
                    _ => (
                        format!("{} = {}\n", variable_name, selected_text.trim()),
                        variable_name.to_string(),
                    ),
                };

                // Find the start of the line
                let line_start = Position {
                    line: range.start.line,
                    character: 0,
                };
                let line_start_byte = self.position_to_byte(source, line_start);
                let line_text = &source[line_start_byte..start_byte];
                let indent = " ".repeat(line_text.len() - line_text.trim_start().len());

                let mut changes = std::collections::HashMap::new();
                changes.insert(uri.clone(), vec![
                    // Insert variable declaration at the start of the line
                    TextEdit {
                        range: Range {
                            start: line_start,
                            end: line_start,
                        },
                        new_text: format!("{}{}", indent, declaration),
                    },
                    // Replace selected text with variable name
                    TextEdit {
                        range,
                        new_text: assignment,
                    },
                ]);

                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: format!("Extract to variable '{}'", variable_name),
                    kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        ..Default::default()
                    }),
                    ..Default::default()
                }));
            }
        }

        Ok(actions)
    }

    /// Helper: Convert byte offset to LSP position
    fn byte_to_position(&self, source: &str, byte_offset: usize) -> Position {
        let mut line = 0;
        let mut character = 0;
        let mut current_offset = 0;

        for ch in source.chars() {
            if current_offset >= byte_offset {
                break;
            }

            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }

            current_offset += ch.len_utf8();
        }

        Position { line, character }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_uri(path: &str) -> Url {
        Url::parse(&format!("file://{}", path)).unwrap()
    }

    fn create_dummy_tree(content: &str) -> Arc<tree_sitter::Tree> {
        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("python").unwrap();
        parser.parse(content, "test.py").unwrap()
    }

    #[test]
    fn test_undefined_variable_quick_fix_python() {
        let provider = CodeActionProvider::new();
        let uri = create_uri("/test.py");
        let content = "result = undefined_var + 10\n";

        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 9 },
                end: Position { line: 0, character: 23 },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("universal-lsp".to_string()),
            message: "Undefined name 'undefined_var'".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        let action = provider.create_undefined_name_fix(&diagnostic, &uri, "python", content);
        assert!(action.is_some());

        if let Some(CodeActionOrCommand::CodeAction(action)) = action {
            assert_eq!(action.title, "Define variable 'undefined_var' before use");
            assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
            assert!(action.edit.is_some());
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_undefined_function_quick_fix_python() {
        let provider = CodeActionProvider::new();
        let uri = create_uri("/test.py");
        let content = "result = undefined_func()\n";

        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 9 },
                end: Position { line: 0, character: 24 },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("universal-lsp".to_string()),
            message: "Undefined name 'undefined_func'".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        let action = provider.create_undefined_name_fix(&diagnostic, &uri, "python", content);
        assert!(action.is_some());

        if let Some(CodeActionOrCommand::CodeAction(action)) = action {
            assert_eq!(action.title, "Define function 'undefined_func'");
            assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
            assert!(action.edit.is_some());
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_undefined_variable_quick_fix_javascript() {
        let provider = CodeActionProvider::new();
        let uri = create_uri("/test.js");
        let content = "const result = unknownVar;\n";

        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 15 },
                end: Position { line: 0, character: 25 },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("universal-lsp".to_string()),
            message: "Undefined name 'unknownVar'".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        let action = provider.create_undefined_name_fix(&diagnostic, &uri, "javascript", content);
        assert!(action.is_some());

        if let Some(CodeActionOrCommand::CodeAction(action)) = action {
            assert_eq!(action.title, "Define variable 'unknownVar'");
            assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
            assert!(action.edit.is_some());
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_undefined_variable_quick_fix_rust() {
        let provider = CodeActionProvider::new();
        let uri = create_uri("/test.rs");
        let content = "let result = missing_var;\n";

        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 13 },
                end: Position { line: 0, character: 24 },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("universal-lsp".to_string()),
            message: "Undefined name 'missing_var'".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        let action = provider.create_undefined_name_fix(&diagnostic, &uri, "rust", content);
        assert!(action.is_some());

        if let Some(CodeActionOrCommand::CodeAction(action)) = action {
            assert_eq!(action.title, "Define variable 'missing_var'");
            assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
            assert!(action.edit.is_some());
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_extract_variable_python() {
        let provider = CodeActionProvider::new();
        let uri = create_uri("/test.py");
        let content = "result = 10 + 20\n";
        let tree = create_dummy_tree(content);

        let range = Range {
            start: Position { line: 0, character: 9 },
            end: Position { line: 0, character: 16 },
        };

        let actions = provider.generic_refactorings(
            &tree,
            content,
            range,
            &uri,
            "python",
        ).unwrap();

        assert!(!actions.is_empty());

        if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
            assert_eq!(action.title, "Extract to variable 'extracted_value'");
            assert_eq!(action.kind, Some(CodeActionKind::REFACTOR_EXTRACT));
            assert!(action.edit.is_some());
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_extract_variable_javascript() {
        let provider = CodeActionProvider::new();
        let uri = create_uri("/test.js");
        let content = "const result = 10 + 20;\n";
        let tree = create_dummy_tree(content);

        let range = Range {
            start: Position { line: 0, character: 15 },
            end: Position { line: 0, character: 22 },
        };

        let actions = provider.generic_refactorings(
            &tree,
            content,
            range,
            &uri,
            "javascript",
        ).unwrap();

        assert!(!actions.is_empty());

        if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
            assert_eq!(action.title, "Extract to variable 'extracted_value'");
            assert_eq!(action.kind, Some(CodeActionKind::REFACTOR_EXTRACT));
            assert!(action.edit.is_some());

            // Check that it uses 'const' for JavaScript
            if let Some(workspace_edit) = &action.edit {
                if let Some(changes) = &workspace_edit.changes {
                    let edits = changes.get(&uri).unwrap();
                    assert!(edits[0].new_text.contains("const"));
                }
            }
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_extract_variable_rust() {
        let provider = CodeActionProvider::new();
        let uri = create_uri("/test.rs");
        let content = "let result = 10 + 20;\n";
        let tree = create_dummy_tree(content);

        let range = Range {
            start: Position { line: 0, character: 13 },
            end: Position { line: 0, character: 20 },
        };

        let actions = provider.generic_refactorings(
            &tree,
            content,
            range,
            &uri,
            "rust",
        ).unwrap();

        assert!(!actions.is_empty());

        if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
            assert_eq!(action.title, "Extract to variable 'extracted_value'");
            assert_eq!(action.kind, Some(CodeActionKind::REFACTOR_EXTRACT));
            assert!(action.edit.is_some());

            // Check that it uses 'let' for Rust
            if let Some(workspace_edit) = &action.edit {
                if let Some(changes) = &workspace_edit.changes {
                    let edits = changes.get(&uri).unwrap();
                    assert!(edits[0].new_text.contains("let"));
                }
            }
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_no_extract_for_multiline_selection() {
        let provider = CodeActionProvider::new();
        let uri = create_uri("/test.py");
        let content = "result = 10 +\n    20\n";
        let tree = create_dummy_tree(content);

        let range = Range {
            start: Position { line: 0, character: 9 },
            end: Position { line: 1, character: 6 },
        };

        let actions = provider.generic_refactorings(
            &tree,
            content,
            range,
            &uri,
            "python",
        ).unwrap();

        // Should not offer extract variable for multiline selections
        assert!(actions.is_empty());
    }

    #[test]
    fn test_position_to_byte_conversion() {
        let provider = CodeActionProvider::new();
        let source = "hello\nworld\n";

        assert_eq!(provider.position_to_byte(source, Position { line: 0, character: 0 }), 0);
        assert_eq!(provider.position_to_byte(source, Position { line: 0, character: 5 }), 5);
        assert_eq!(provider.position_to_byte(source, Position { line: 1, character: 0 }), 6);
        assert_eq!(provider.position_to_byte(source, Position { line: 1, character: 5 }), 11);
    }

    #[test]
    fn test_byte_to_position_conversion() {
        let provider = CodeActionProvider::new();
        let source = "hello\nworld\n";

        assert_eq!(provider.byte_to_position(source, 0), Position { line: 0, character: 0 });
        assert_eq!(provider.byte_to_position(source, 5), Position { line: 0, character: 5 });
        assert_eq!(provider.byte_to_position(source, 6), Position { line: 1, character: 0 });
        assert_eq!(provider.byte_to_position(source, 11), Position { line: 1, character: 5 });
    }

    #[test]
    fn test_ai_actions_with_selection() {
        use crate::ai::claude::{ClaudeClient, ClaudeConfig};

        // Create a provider with Claude client
        let config = ClaudeConfig {
            api_key: "test-key".to_string(),
            ..Default::default()
        };
        let claude_client = ClaudeClient::new(config).ok().map(Arc::new);
        let provider = CodeActionProvider::with_claude(claude_client);

        let uri = create_uri("/test.py");
        let content = "def hello():\n    print('Hello')\n";

        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 1, character: 19 },
        };

        let actions = provider.get_ai_actions(content, range, &uri, "python").unwrap();

        // Should have 4 AI actions: explain, optimize, generate tests, generate docs
        assert_eq!(actions.len(), 4);

        // Check action titles
        let titles: Vec<String> = actions.iter().filter_map(|a| {
            if let CodeActionOrCommand::Command(cmd) = a {
                Some(cmd.title.clone())
            } else {
                None
            }
        }).collect();

        assert!(titles.iter().any(|t| t.contains("Explain code")));
        assert!(titles.iter().any(|t| t.contains("Optimize code")));
        assert!(titles.iter().any(|t| t.contains("Generate tests")));
        assert!(titles.iter().any(|t| t.contains("Generate documentation")));
    }

    #[test]
    fn test_no_ai_actions_without_selection() {
        use crate::ai::claude::{ClaudeClient, ClaudeConfig};

        let config = ClaudeConfig {
            api_key: "test-key".to_string(),
            ..Default::default()
        };
        let claude_client = ClaudeClient::new(config).ok().map(Arc::new);
        let provider = CodeActionProvider::with_claude(claude_client);

        let uri = create_uri("/test.py");
        let content = "def hello():\n    print('Hello')\n";

        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 },
        };

        let actions = provider.get_ai_actions(content, range, &uri, "python").unwrap();

        // Should have no AI actions when there's no selection
        assert_eq!(actions.len(), 0);
    }
}
