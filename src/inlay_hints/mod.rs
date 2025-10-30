//! Inlay Hints Module
//!
//! Provides inline annotations for parameter names, types, and other contextual information

use anyhow::Result;
use tower_lsp::lsp_types::*;
use crate::tree_sitter::TreeSitterParser;

/// Type of inlay hint
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintKind {
    /// Parameter name hint
    Parameter,
    /// Type annotation hint
    Type,
}

/// Inlay hints provider
#[derive(Debug)]
pub struct InlayHintsProvider {
    /// Show parameter name hints
    show_parameter_hints: bool,
    /// Show type hints
    show_type_hints: bool,
}

impl InlayHintsProvider {
    pub fn new() -> Self {
        Self {
            show_parameter_hints: true,
            show_type_hints: true,
        }
    }

    /// Configure which hints to show
    pub fn with_config(mut self, show_parameter_hints: bool, show_type_hints: bool) -> Self {
        self.show_parameter_hints = show_parameter_hints;
        self.show_type_hints = show_type_hints;
        self
    }

    /// Get inlay hints for a range in the document
    pub fn get_inlay_hints(
        &self,
        content: &str,
        range: Range,
        lang: &str,
    ) -> Result<Vec<InlayHint>> {
        let mut parser = TreeSitterParser::new()?;
        if parser.set_language(lang).is_err() {
            return Ok(Vec::new());
        }

        let tree = parser.parse(content, "temp")?;
        let root = tree.root_node();

        let mut hints = Vec::new();

        // Find all nodes in the range
        let start_byte = self.position_to_byte(content, range.start);
        let end_byte = self.position_to_byte(content, range.end);

        let mut cursor = root.walk();
        self.collect_hints_recursive(root, content, lang, start_byte, end_byte, &mut hints, &mut cursor)?;

        Ok(hints)
    }

    /// Recursively collect hints from tree
    fn collect_hints_recursive(
        &self,
        node: tree_sitter::Node,
        content: &str,
        lang: &str,
        start_byte: usize,
        end_byte: usize,
        hints: &mut Vec<InlayHint>,
        cursor: &mut tree_sitter::TreeCursor,
    ) -> Result<()> {
        // Check if node is in range
        if node.start_byte() > end_byte || node.end_byte() < start_byte {
            return Ok(());
        }

        // Collect hints for this node
        match lang {
            "python" => self.collect_python_hints(&node, content, hints),
            "javascript" | "typescript" | "tsx" | "jsx" => self.collect_js_hints(&node, content, hints),
            "rust" => self.collect_rust_hints(&node, content, hints),
            _ => {}
        }

        // Process children
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                self.collect_hints_recursive(child, content, lang, start_byte, end_byte, hints, cursor)?;

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }

        Ok(())
    }

    /// Collect hints for Python code
    fn collect_python_hints(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        hints: &mut Vec<InlayHint>,
    ) {
        match node.kind() {
            "call" => {
                if self.show_parameter_hints {
                    self.add_python_parameter_hints(node, content, hints);
                }
            }
            "assignment" => {
                if self.show_type_hints {
                    self.add_python_type_hints(node, content, hints);
                }
            }
            _ => {}
        }
    }

    /// Add parameter name hints for Python function calls
    fn add_python_parameter_hints(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        hints: &mut Vec<InlayHint>,
    ) {
        // Get the arguments node
        if let Some(args_node) = node.child_by_field_name("arguments") {
            let mut cursor = args_node.walk();
            let mut param_index = 0;

            for child in args_node.children(&mut cursor) {
                // Skip keyword arguments (they already have names)
                if child.kind() == "keyword_argument" {
                    continue;
                }

                // Add hint for positional arguments
                if child.is_named() && child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                    let position = self.byte_to_position(content, child.start_byte());

                    hints.push(InlayHint {
                        position,
                        label: InlayHintLabel::String(format!("param{}:", param_index)),
                        kind: Some(InlayHintKind::PARAMETER),
                        text_edits: None,
                        tooltip: None,
                        padding_left: Some(false),
                        padding_right: Some(true),
                        data: None,
                    });

                    param_index += 1;
                }
            }
        }
    }

    /// Add type hints for Python variables
    fn add_python_type_hints(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        hints: &mut Vec<InlayHint>,
    ) {
        // Look for assignments without type annotations
        if let Some(left) = node.child_by_field_name("left") {
            if left.kind() == "identifier" {
                // Check if there's already a type annotation
                let has_annotation = node.children(&mut node.walk())
                    .any(|c| c.kind() == "type");

                if !has_annotation {
                    if let Some(right) = node.child_by_field_name("right") {
                        let inferred_type = self.infer_python_type(&right, content);
                        if let Some(type_str) = inferred_type {
                            let position = self.byte_to_position(content, left.end_byte());

                            hints.push(InlayHint {
                                position,
                                label: InlayHintLabel::String(format!(": {}", type_str)),
                                kind: Some(InlayHintKind::TYPE),
                                text_edits: None,
                                tooltip: None,
                                padding_left: Some(false),
                                padding_right: Some(false),
                                data: None,
                            });
                        }
                    }
                }
            }
        }
    }

    /// Infer Python type from expression
    fn infer_python_type(&self, node: &tree_sitter::Node, content: &str) -> Option<String> {
        match node.kind() {
            "integer" => Some("int".to_string()),
            "float" => Some("float".to_string()),
            "string" | "string_content" => Some("str".to_string()),
            "true" | "false" => Some("bool".to_string()),
            "list" => Some("list".to_string()),
            "dictionary" => Some("dict".to_string()),
            "tuple" => Some("tuple".to_string()),
            "set" => Some("set".to_string()),
            "call" => {
                // Try to infer from function name
                if let Some(func) = node.child_by_field_name("function") {
                    let func_name = &content[func.start_byte()..func.end_byte()];
                    match func_name {
                        "int" => Some("int".to_string()),
                        "float" => Some("float".to_string()),
                        "str" => Some("str".to_string()),
                        "list" => Some("list".to_string()),
                        "dict" => Some("dict".to_string()),
                        "set" => Some("set".to_string()),
                        "tuple" => Some("tuple".to_string()),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Collect hints for JavaScript/TypeScript code
    fn collect_js_hints(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        hints: &mut Vec<InlayHint>,
    ) {
        match node.kind() {
            "call_expression" => {
                if self.show_parameter_hints {
                    self.add_js_parameter_hints(node, content, hints);
                }
            }
            "variable_declarator" => {
                if self.show_type_hints {
                    self.add_js_type_hints(node, content, hints);
                }
            }
            _ => {}
        }
    }

    /// Add parameter name hints for JavaScript function calls
    fn add_js_parameter_hints(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        hints: &mut Vec<InlayHint>,
    ) {
        if let Some(args_node) = node.child_by_field_name("arguments") {
            let mut cursor = args_node.walk();
            let mut param_index = 0;

            for child in args_node.children(&mut cursor) {
                if child.is_named() && child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                    let position = self.byte_to_position(content, child.start_byte());

                    hints.push(InlayHint {
                        position,
                        label: InlayHintLabel::String(format!("param{}:", param_index)),
                        kind: Some(InlayHintKind::PARAMETER),
                        text_edits: None,
                        tooltip: None,
                        padding_left: Some(false),
                        padding_right: Some(true),
                        data: None,
                    });

                    param_index += 1;
                }
            }
        }
    }

    /// Add type hints for JavaScript variables
    fn add_js_type_hints(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        hints: &mut Vec<InlayHint>,
    ) {
        if let Some(name) = node.child_by_field_name("name") {
            // Check if there's already a type annotation
            let has_annotation = node.children(&mut node.walk())
                .any(|c| c.kind() == "type_annotation");

            if !has_annotation {
                if let Some(value) = node.child_by_field_name("value") {
                    let inferred_type = self.infer_js_type(&value, content);
                    if let Some(type_str) = inferred_type {
                        let position = self.byte_to_position(content, name.end_byte());

                        hints.push(InlayHint {
                            position,
                            label: InlayHintLabel::String(format!(": {}", type_str)),
                            kind: Some(InlayHintKind::TYPE),
                            text_edits: None,
                            tooltip: None,
                            padding_left: Some(false),
                            padding_right: Some(false),
                            data: None,
                        });
                    }
                }
            }
        }
    }

    /// Infer JavaScript type from expression
    fn infer_js_type(&self, node: &tree_sitter::Node, _content: &str) -> Option<String> {
        match node.kind() {
            "number" => Some("number".to_string()),
            "string" | "template_string" => Some("string".to_string()),
            "true" | "false" => Some("boolean".to_string()),
            "array" => Some("Array".to_string()),
            "object" => Some("Object".to_string()),
            "arrow_function" | "function" => Some("Function".to_string()),
            "null" => Some("null".to_string()),
            "undefined" => Some("undefined".to_string()),
            _ => None,
        }
    }

    /// Collect hints for Rust code
    fn collect_rust_hints(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        hints: &mut Vec<InlayHint>,
    ) {
        match node.kind() {
            "call_expression" => {
                if self.show_parameter_hints {
                    self.add_rust_parameter_hints(node, content, hints);
                }
            }
            "let_declaration" => {
                if self.show_type_hints {
                    self.add_rust_type_hints(node, content, hints);
                }
            }
            _ => {}
        }
    }

    /// Add parameter name hints for Rust function calls
    fn add_rust_parameter_hints(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        hints: &mut Vec<InlayHint>,
    ) {
        if let Some(args_node) = node.child_by_field_name("arguments") {
            let mut cursor = args_node.walk();
            let mut param_index = 0;

            for child in args_node.children(&mut cursor) {
                if child.is_named() && child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                    let position = self.byte_to_position(content, child.start_byte());

                    hints.push(InlayHint {
                        position,
                        label: InlayHintLabel::String(format!("param{}:", param_index)),
                        kind: Some(InlayHintKind::PARAMETER),
                        text_edits: None,
                        tooltip: None,
                        padding_left: Some(false),
                        padding_right: Some(true),
                        data: None,
                    });

                    param_index += 1;
                }
            }
        }
    }

    /// Add type hints for Rust variables
    fn add_rust_type_hints(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        hints: &mut Vec<InlayHint>,
    ) {
        // Check if there's already a type annotation
        let has_type = node.child_by_field_name("type").is_some();

        if !has_type {
            if let Some(pattern) = node.child_by_field_name("pattern") {
                if let Some(value) = node.child_by_field_name("value") {
                    let inferred_type = self.infer_rust_type(&value, content);
                    if let Some(type_str) = inferred_type {
                        let position = self.byte_to_position(content, pattern.end_byte());

                        hints.push(InlayHint {
                            position,
                            label: InlayHintLabel::String(format!(": {}", type_str)),
                            kind: Some(InlayHintKind::TYPE),
                            text_edits: None,
                            tooltip: None,
                            padding_left: Some(false),
                            padding_right: Some(false),
                            data: None,
                        });
                    }
                }
            }
        }
    }

    /// Infer Rust type from expression
    fn infer_rust_type(&self, node: &tree_sitter::Node, content: &str) -> Option<String> {
        match node.kind() {
            "integer_literal" => {
                let text = &content[node.start_byte()..node.end_byte()];
                if text.ends_with("u32") || text.ends_with("i32") || text.ends_with("u64") || text.ends_with("i64") {
                    None // Already has type suffix
                } else {
                    Some("i32".to_string())
                }
            }
            "float_literal" => Some("f64".to_string()),
            "string_literal" | "raw_string_literal" => Some("&str".to_string()),
            "char_literal" => Some("char".to_string()),
            "true" | "false" => Some("bool".to_string()),
            "array_expression" => Some("Vec".to_string()),
            _ => None,
        }
    }

    /// Convert LSP position to byte offset
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

    /// Convert byte offset to LSP position
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
}

impl Default for InlayHintsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_inlay_hints() {
        let provider = InlayHintsProvider::new();
        let content = r#"
def calculate_sum(a, b, c):
    return a + b + c

result = calculate_sum(1, 2, 3)
x = 42
y = "hello"
"#;

        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 10, character: 0 },
        };

        let hints = provider.get_inlay_hints(content, range, "python").unwrap();
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_javascript_inlay_hints() {
        let provider = InlayHintsProvider::new();
        let content = r#"
function add(x, y) {
    return x + y;
}

const result = add(5, 10);
let count = 42;
const name = "test";
"#;

        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 10, character: 0 },
        };

        let hints = provider.get_inlay_hints(content, range, "javascript").unwrap();
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_rust_inlay_hints() {
        let provider = InlayHintsProvider::new();
        let content = r#"
fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

fn main() {
    let result = multiply(3, 4);
    let count = 42;
    let message = "hello";
}
"#;

        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 12, character: 0 },
        };

        let hints = provider.get_inlay_hints(content, range, "rust").unwrap();
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_parameter_hints_only() {
        let provider = InlayHintsProvider::new().with_config(true, false);
        let content = "result = add(5, 10)";

        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 1, character: 0 },
        };

        let hints = provider.get_inlay_hints(content, range, "python").unwrap();
        // Should only have parameter hints, no type hints
        for hint in hints {
            assert_eq!(hint.kind, Some(InlayHintKind::PARAMETER));
        }
    }

    #[test]
    fn test_type_hints_only() {
        let provider = InlayHintsProvider::new().with_config(false, true);
        let content = "x = 42";

        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 1, character: 0 },
        };

        let hints = provider.get_inlay_hints(content, range, "python").unwrap();
        // Should only have type hints, no parameter hints
        for hint in hints {
            assert_eq!(hint.kind, Some(InlayHintKind::TYPE));
        }
    }

    #[test]
    fn test_empty_content() {
        let provider = InlayHintsProvider::new();
        let content = "";

        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 },
        };

        let hints = provider.get_inlay_hints(content, range, "python").unwrap();
        assert!(hints.is_empty());
    }

    #[test]
    fn test_unsupported_language() {
        let provider = InlayHintsProvider::new();
        let content = "some code here";

        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 1, character: 0 },
        };

        let hints = provider.get_inlay_hints(content, range, "unsupported").unwrap();
        assert!(hints.is_empty());
    }

    #[test]
    fn test_position_conversion() {
        let provider = InlayHintsProvider::new();
        let content = "line1\nline2\nline3";

        let byte_offset = provider.position_to_byte(content, Position { line: 1, character: 3 });
        let position = provider.byte_to_position(content, byte_offset);

        assert_eq!(position.line, 1);
        assert_eq!(position.character, 3);
    }
}
