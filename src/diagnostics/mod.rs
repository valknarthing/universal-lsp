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
fn analyze_js_semantics(tree: &Tree, source: &str) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // Find undefined variables (simple heuristic: used but never defined)
    let mut defined_names = std::collections::HashSet::new();
    let mut used_names = Vec::new();

    let mut cursor = tree.root_node().walk();
    collect_js_names(&mut cursor, source, &mut defined_names, &mut used_names);

    // Check for undefined names
    for (name, pos) in used_names {
        if !defined_names.contains(&name) && !is_js_builtin(&name) {
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

/// Collect JavaScript/TypeScript variable definitions and usages
fn collect_js_names(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    defined: &mut std::collections::HashSet<String>,
    used: &mut Vec<(String, Position)>,
) {
    let node = cursor.node();

    match node.kind() {
        // Definition sites
        "function_declaration" | "function" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    defined.insert(name.to_string());
                }
            }
        }
        "class_declaration" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    defined.insert(name.to_string());
                }
            }
        }
        "variable_declarator" => {
            // const/let/var declarations
            if let Some(name_node) = node.child_by_field_name("name") {
                if name_node.kind() == "identifier" {
                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                        defined.insert(name.to_string());
                    }
                }
            }
        }
        "formal_parameters" => {
            // function parameters are definitions
            let mut param_cursor = node.walk();
            for child in node.children(&mut param_cursor) {
                if child.kind() == "identifier" || child.kind() == "required_parameter" {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        defined.insert(name.to_string());
                    }
                }
            }
        }
        "import_specifier" => {
            // import { foo } from 'module'
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    defined.insert(name.to_string());
                }
            }
        }
        // Usage sites
        "identifier" => {
            // Check if this identifier is being used (not defined)
            if let Some(parent) = node.parent() {
                let is_definition = matches!(
                    parent.kind(),
                    "function_declaration" | "class_declaration" | "variable_declarator"
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
            collect_js_names(cursor, source, defined, used);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Check if a name is a JavaScript/TypeScript builtin
fn is_js_builtin(name: &str) -> bool {
    matches!(
        name,
        // Global objects
        "Object"
            | "Array"
            | "String"
            | "Number"
            | "Boolean"
            | "Function"
            | "Symbol"
            | "BigInt"
            | "Math"
            | "Date"
            | "RegExp"
            | "Error"
            | "JSON"
            | "Promise"
            | "Map"
            | "Set"
            | "WeakMap"
            | "WeakSet"
            // Global functions
            | "console"
            | "parseInt"
            | "parseFloat"
            | "isNaN"
            | "isFinite"
            | "encodeURI"
            | "encodeURIComponent"
            | "decodeURI"
            | "decodeURIComponent"
            | "eval"
            | "setTimeout"
            | "setInterval"
            | "clearTimeout"
            | "clearInterval"
            // Common globals
            | "window"
            | "document"
            | "navigator"
            | "location"
            | "fetch"
            | "require"
            | "module"
            | "exports"
            | "process"
            | "global"
            | "__dirname"
            | "__filename"
            // Keywords/literals
            | "undefined"
            | "null"
            | "true"
            | "false"
            | "this"
            | "arguments"
            | "Infinity"
            | "NaN"
    )
}

/// Analyze Rust semantic errors
fn analyze_rust_semantics(tree: &Tree, source: &str) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // Find undefined variables/items
    let mut defined_names = std::collections::HashSet::new();
    let mut used_names = Vec::new();

    let mut cursor = tree.root_node().walk();
    collect_rust_names(&mut cursor, source, &mut defined_names, &mut used_names);

    // Check for undefined names
    for (name, pos) in used_names {
        if !defined_names.contains(&name) && !is_rust_builtin(&name) {
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

/// Collect Rust variable definitions and usages
fn collect_rust_names(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    defined: &mut std::collections::HashSet<String>,
    used: &mut Vec<(String, Position)>,
) {
    let node = cursor.node();

    match node.kind() {
        // Definition sites
        "function_item" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    defined.insert(name.to_string());
                }
            }
        }
        "struct_item" | "enum_item" | "type_item" | "trait_item" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    defined.insert(name.to_string());
                }
            }
        }
        "let_declaration" => {
            // let bindings
            if let Some(pattern) = node.child_by_field_name("pattern") {
                if pattern.kind() == "identifier" {
                    if let Ok(name) = pattern.utf8_text(source.as_bytes()) {
                        defined.insert(name.to_string());
                    }
                }
            }
        }
        "parameters" | "parameter" => {
            // function parameters
            let mut param_cursor = node.walk();
            for child in node.children(&mut param_cursor) {
                if child.kind() == "identifier" {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        defined.insert(name.to_string());
                    }
                }
            }
        }
        "use_declaration" => {
            // use statements import names
            let mut use_cursor = node.walk();
            for child in node.children(&mut use_cursor) {
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
                    "function_item"
                        | "struct_item"
                        | "enum_item"
                        | "type_item"
                        | "trait_item"
                        | "let_declaration"
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
            collect_rust_names(cursor, source, defined, used);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Check if a name is a Rust standard library item or keyword
fn is_rust_builtin(name: &str) -> bool {
    matches!(
        name,
        // Primitive types
        "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "str"
            // Common types
            | "String"
            | "Vec"
            | "Option"
            | "Some"
            | "None"
            | "Result"
            | "Ok"
            | "Err"
            | "Box"
            | "Rc"
            | "Arc"
            | "Cell"
            | "RefCell"
            // Traits
            | "Clone"
            | "Copy"
            | "Debug"
            | "Default"
            | "Drop"
            | "Send"
            | "Sync"
            | "Sized"
            | "Iterator"
            | "IntoIterator"
            | "From"
            | "Into"
            // Macros (without !)
            | "println"
            | "print"
            | "eprintln"
            | "eprint"
            | "dbg"
            | "panic"
            | "assert"
            | "assert_eq"
            | "assert_ne"
            | "vec"
            | "format"
            // Keywords
            | "self"
            | "Self"
            | "super"
            | "crate"
            | "true"
            | "false"
    )
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
    use crate::tree_sitter::TreeSitterParser;

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

    #[tokio::test]
    async fn test_syntax_error_detection() {
        let source = r#"
def broken_function():
    print("missing closing paren"
"#;

        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("python").unwrap();
        let tree = parser.parse(source, "test.py").unwrap();

        let diagnostics = compute_diagnostics(&tree, source, "python", None).await.unwrap();

        // Should detect syntax error
        assert!(!diagnostics.is_empty(), "Should detect syntax error for unclosed parenthesis");
        assert!(diagnostics.iter().any(|d| d.severity == Some(DiagnosticSeverity::ERROR)));
    }

    #[tokio::test]
    async fn test_undefined_variable_detection() {
        let source = r#"
def test_function():
    result = undefined_variable + 10
    return result
"#;

        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("python").unwrap();
        let tree = parser.parse(source, "test.py").unwrap();

        let diagnostics = compute_diagnostics(&tree, source, "python", None).await.unwrap();

        // Should detect undefined variable
        assert!(diagnostics.iter().any(|d| {
            d.message.contains("undefined_variable") &&
            d.severity == Some(DiagnosticSeverity::WARNING)
        }), "Should detect undefined variable");
    }

    #[tokio::test]
    async fn test_no_false_positives_for_builtins() {
        let source = r#"
def test_builtins():
    print(len([1, 2, 3]))
    result = str(123)
    return result
"#;

        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("python").unwrap();
        let tree = parser.parse(source, "test.py").unwrap();

        let diagnostics = compute_diagnostics(&tree, source, "python", None).await.unwrap();

        // Should NOT report print, len, or str as undefined
        assert!(!diagnostics.iter().any(|d| d.message.contains("print")));
        assert!(!diagnostics.iter().any(|d| d.message.contains("len")));
        assert!(!diagnostics.iter().any(|d| d.message.contains("str")));
    }

    #[tokio::test]
    async fn test_defined_variables_no_warning() {
        let source = r#"
def calculate_sum(a, b):
    result = a + b
    return result

x = calculate_sum(5, 3)
print(x)
"#;

        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("python").unwrap();
        let tree = parser.parse(source, "test.py").unwrap();

        let diagnostics = compute_diagnostics(&tree, source, "python", None).await.unwrap();

        // Should NOT report any undefined variables
        assert!(!diagnostics.iter().any(|d| d.message.contains("Undefined name")));
    }

    #[tokio::test]
    async fn test_multiple_errors() {
        let source = r#"
def test():
    x = undefined_var1
    y = undefined_var2
    return x + y
"#;

        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("python").unwrap();
        let tree = parser.parse(source, "test.py").unwrap();

        let diagnostics = compute_diagnostics(&tree, source, "python", None).await.unwrap();

        // Should detect both undefined variables
        assert!(diagnostics.iter().any(|d| d.message.contains("undefined_var1")));
        assert!(diagnostics.iter().any(|d| d.message.contains("undefined_var2")));
    }

    // JavaScript/TypeScript tests

    #[tokio::test]
    async fn test_js_undefined_variable() {
        let source = r#"
function test() {
    const result = undefinedVar + 10;
    return result;
}
"#;

        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("javascript").unwrap();
        let tree = parser.parse(source, "test.js").unwrap();

        let diagnostics = compute_diagnostics(&tree, source, "javascript", None).await.unwrap();

        // Should detect undefined variable
        assert!(diagnostics.iter().any(|d| {
            d.message.contains("undefinedVar") &&
            d.severity == Some(DiagnosticSeverity::WARNING)
        }), "Should detect undefined JavaScript variable");
    }

    #[tokio::test]
    async fn test_js_no_false_positives_for_builtins() {
        let source = r#"
function test() {
    console.log(Array.from([1, 2, 3]));
    const obj = JSON.parse('{"key": "value"}');
    return Promise.resolve(obj);
}
"#;

        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("javascript").unwrap();
        let tree = parser.parse(source, "test.js").unwrap();

        let diagnostics = compute_diagnostics(&tree, source, "javascript", None).await.unwrap();

        // Should NOT report console, Array, JSON, Promise as undefined
        assert!(!diagnostics.iter().any(|d| d.message.contains("console")));
        assert!(!diagnostics.iter().any(|d| d.message.contains("Array")));
        assert!(!diagnostics.iter().any(|d| d.message.contains("JSON")));
        assert!(!diagnostics.iter().any(|d| d.message.contains("Promise")));
    }

    #[tokio::test]
    async fn test_js_defined_variables_no_warning() {
        let source = r#"
function calculateSum(a, b) {
    const result = a + b;
    return result;
}

const x = calculateSum(5, 3);
console.log(x);
"#;

        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("javascript").unwrap();
        let tree = parser.parse(source, "test.js").unwrap();

        let diagnostics = compute_diagnostics(&tree, source, "javascript", None).await.unwrap();

        // Should NOT report any undefined variables
        assert!(!diagnostics.iter().any(|d| d.message.contains("Undefined name")));
    }

    // Rust tests

    #[tokio::test]
    async fn test_rust_undefined_variable() {
        let source = r#"
fn test() {
    let result = undefined_var + 10;
    result
}
"#;

        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("rust").unwrap();
        let tree = parser.parse(source, "test.rs").unwrap();

        let diagnostics = compute_diagnostics(&tree, source, "rust", None).await.unwrap();

        // Should detect undefined variable
        assert!(diagnostics.iter().any(|d| {
            d.message.contains("undefined_var") &&
            d.severity == Some(DiagnosticSeverity::WARNING)
        }), "Should detect undefined Rust variable");
    }

    #[tokio::test]
    async fn test_rust_no_false_positives_for_builtins() {
        let source = r#"
fn test() {
    println!("Hello");
    let vec = Vec::new();
    let opt = Some(42);
    let result = Ok(());
}
"#;

        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("rust").unwrap();
        let tree = parser.parse(source, "test.rs").unwrap();

        let diagnostics = compute_diagnostics(&tree, source, "rust", None).await.unwrap();

        // Should NOT report println, Vec, Some, Ok as undefined
        assert!(!diagnostics.iter().any(|d| d.message.contains("println")));
        assert!(!diagnostics.iter().any(|d| d.message.contains("Vec")));
        assert!(!diagnostics.iter().any(|d| d.message.contains("Some")));
        assert!(!diagnostics.iter().any(|d| d.message.contains("Ok")));
    }

    #[tokio::test]
    async fn test_rust_defined_variables_no_warning() {
        let source = r#"
fn calculate_sum(a: i32, b: i32) -> i32 {
    let result = a + b;
    result
}

fn main() {
    let x = calculate_sum(5, 3);
    println!("{}", x);
}
"#;

        let mut parser = TreeSitterParser::new().unwrap();
        parser.set_language("rust").unwrap();
        let tree = parser.parse(source, "test.rs").unwrap();

        let diagnostics = compute_diagnostics(&tree, source, "rust", None).await.unwrap();

        // Should NOT report any undefined variables
        assert!(!diagnostics.iter().any(|d| d.message.contains("Undefined name")));
    }
}
