//! Diagnostics module for Universal LSP
//!
//! Provides real-time error detection and code quality analysis through:
//! - Syntax errors from tree-sitter error nodes
//! - Semantic analysis (undefined symbols, type errors)
//! - AI-enhanced diagnostics via Claude

use anyhow::Result;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tree_sitter::Tree;

use crate::ai::claude::ClaudeClient;

/// Diagnostic provider for computing diagnostics
pub struct DiagnosticProvider {}

impl DiagnosticProvider {
    /// Create a new diagnostic provider
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for DiagnosticProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute all diagnostics for a document
pub async fn compute_diagnostics(
    tree: &Tree,
    source: &str,
    lang: &str,
    _claude_client: Option<&ClaudeClient>,
) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // 1. Extract syntax errors from tree-sitter
    diagnostics.extend(extract_syntax_errors(tree, source));

    // 2. Semantic analysis (undefined symbols, etc.)
    diagnostics.extend(analyze_semantic_errors(tree, source, lang)?);

    // 3. AI-enhanced diagnostics (future)
    // if let Some(claude) = claude_client {
    //     diagnostics.extend(ai_analyze(source, lang, claude).await?);
    // }

    Ok(diagnostics)
}

/// Extract syntax errors from tree-sitter error nodes
fn extract_syntax_errors(tree: &Tree, source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let root = tree.root_node();

    // Walk the tree and find ERROR nodes
    let mut cursor = root.walk();
    visit_errors(&mut cursor, source, &mut diagnostics);

    diagnostics
}

/// Recursively visit tree nodes to find errors
fn visit_errors(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let node = cursor.node();

    // Check if this node is an error
    if node.is_error() || node.kind() == "ERROR" {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();

        let start_pos = byte_to_position(source, start_byte);
        let end_pos = byte_to_position(source, end_byte);

        // Get error context (surrounding text)
        let error_text = if start_byte < source.len() && end_byte <= source.len() {
            &source[start_byte..end_byte.min(start_byte + 50)]
        } else {
            "<invalid>"
        };

        diagnostics.push(Diagnostic {
            range: Range {
                start: start_pos,
                end: end_pos,
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("universal-lsp".to_string()),
            message: format!("Syntax error: unexpected token '{}'", error_text),
            related_information: None,
            tags: None,
            data: None,
        });
    }

    // Check for missing nodes (indicated by tree-sitter with is_missing())
    if node.is_missing() {
        let start_byte = node.start_byte();
        let start_pos = byte_to_position(source, start_byte);

        diagnostics.push(Diagnostic {
            range: Range {
                start: start_pos,
                end: Position {
                    line: start_pos.line,
                    character: start_pos.character + 1,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("universal-lsp".to_string()),
            message: format!("Missing: expected {}", node.kind()),
            related_information: None,
            tags: None,
            data: None,
        });
    }

    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            visit_errors(cursor, source, diagnostics);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Analyze semantic errors (undefined symbols, type mismatches, etc.)
fn analyze_semantic_errors(tree: &Tree, source: &str, lang: &str) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    match lang {
        "python" => {
            diagnostics.extend(analyze_python_semantics(tree, source)?);
        }
        "javascript" | "typescript" | "tsx" => {
            diagnostics.extend(analyze_js_semantics(tree, source)?);
        }
        "rust" => {
            diagnostics.extend(analyze_rust_semantics(tree, source)?);
        }
        _ => {
            // Generic semantic analysis for other languages
        }
    }

    Ok(diagnostics)
}

/// Analyze Python-specific semantic errors
fn analyze_python_semantics(tree: &Tree, source: &str) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // Find undefined variables (simple heuristic: used but never defined)
    let mut defined_names = std::collections::HashSet::new();
    let mut used_names = Vec::new();

    let mut cursor = tree.root_node().walk();
    collect_python_names(&mut cursor, source, &mut defined_names, &mut used_names);

    // Check for undefined names
    for (name, pos) in used_names {
        if !defined_names.contains(&name) && !is_python_builtin(&name) {
            diagnostics.push(Diagnostic {
                range: Range {
                    start: pos,
                    end: Position {
                        line: pos.line,
                        character: pos.character + name.len() as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                code: None,
                code_description: None,
                source: Some("universal-lsp".to_string()),
                message: format!("Undefined name '{}'", name),
                related_information: None,
                tags: None,
                data: None,
            });
        }
    }

    Ok(diagnostics)
}

/// Collect Python variable definitions and usages
fn collect_python_names(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    defined: &mut std::collections::HashSet<String>,
    used: &mut Vec<(String, Position)>,
) {
    let node = cursor.node();

    match node.kind() {
        // Definition sites
        "function_definition" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    defined.insert(name.to_string());
                }
            }
        }
        "class_definition" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    defined.insert(name.to_string());
                }
            }
        }
        "assignment" => {
            // left side of assignment
            if let Some(left) = node.child_by_field_name("left") {
                if left.kind() == "identifier" {
                    if let Ok(name) = left.utf8_text(source.as_bytes()) {
                        defined.insert(name.to_string());
                    }
                }
            }
        }
        "parameters" => {
            // function parameters are definitions
            let mut param_cursor = node.walk();
            for child in node.children(&mut param_cursor) {
                if child.kind() == "identifier" {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        defined.insert(name.to_string());
                    }
                }
            }
        }
        // Usage sites
        "identifier" => {
            // Check if this identifier is being used (not defined)
            if let Some(parent) = node.parent() {
                let is_definition = matches!(
                    parent.kind(),
                    "function_definition" | "class_definition"
                ) && parent.child_by_field_name("name").map(|n| n.id()) == Some(node.id());

                if !is_definition {
                    if let Ok(name) = node.utf8_text(source.as_bytes()) {
                        let pos = byte_to_position(source, node.start_byte());
                        used.push((name.to_string(), pos));
                    }
                }
            }
        }
        _ => {}
    }

    // Recurse
    if cursor.goto_first_child() {
        loop {
            collect_python_names(cursor, source, defined, used);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Check if a name is a Python builtin
fn is_python_builtin(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "len"
            | "str"
            | "int"
            | "float"
            | "list"
            | "dict"
            | "set"
            | "tuple"
            | "range"
            | "enumerate"
            | "zip"
            | "map"
            | "filter"
            | "sum"
            | "min"
            | "max"
            | "abs"
            | "round"
            | "sorted"
            | "reversed"
            | "any"
            | "all"
            | "open"
            | "input"
            | "type"
            | "isinstance"
            | "hasattr"
            | "getattr"
            | "setattr"
            | "delattr"
            | "dir"
            | "help"
            | "id"
            | "hash"
            | "hex"
            | "oct"
            | "bin"
            | "chr"
            | "ord"
            | "format"
            | "object"
            | "property"
            | "staticmethod"
            | "classmethod"
            | "super"
            | "Exception"
            | "ValueError"
            | "TypeError"
            | "KeyError"
            | "IndexError"
            | "RuntimeError"
            | "True"
            | "False"
            | "None"
    )
}

/// Analyze JavaScript/TypeScript semantic errors
fn analyze_js_semantics(_tree: &Tree, _source: &str) -> Result<Vec<Diagnostic>> {
    // TODO: Implement JS-specific semantic analysis
    Ok(Vec::new())
}

/// Analyze Rust semantic errors
fn analyze_rust_semantics(_tree: &Tree, _source: &str) -> Result<Vec<Diagnostic>> {
    // TODO: Implement Rust-specific semantic analysis
    Ok(Vec::new())
}

/// Convert byte offset to LSP Position
fn byte_to_position(source: &str, byte_offset: usize) -> Position {
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

    Position { line, character }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_to_position() {
        let source = "hello\nworld\n";
        assert_eq!(byte_to_position(source, 0), Position { line: 0, character: 0 });
        assert_eq!(byte_to_position(source, 6), Position { line: 1, character: 0 });
        assert_eq!(byte_to_position(source, 7), Position { line: 1, character: 1 });
    }

    #[test]
    fn test_is_python_builtin() {
        assert!(is_python_builtin("print"));
        assert!(is_python_builtin("len"));
        assert!(is_python_builtin("True"));
        assert!(!is_python_builtin("my_function"));
    }
}
