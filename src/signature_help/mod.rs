//! Signature Help Module
//!
//! Provides function signature hints with parameter information

use anyhow::Result;
use tower_lsp::lsp_types::*;
use crate::tree_sitter::TreeSitterParser;

/// Signature help provider
#[derive(Debug)]
pub struct SignatureHelpProvider {}

impl SignatureHelpProvider {
    pub fn new() -> Self {
        Self {}
    }

    /// Get signature help at a given position
    pub fn get_signature_help(
        &self,
        content: &str,
        position: Position,
        lang: &str,
    ) -> Result<Option<SignatureHelp>> {
        let mut parser = TreeSitterParser::new()?;
        if parser.set_language(lang).is_err() {
            return Ok(None);
        }

        let tree = parser.parse(content, "temp")?;
        let byte_offset = self.position_to_byte(content, position);

        // Find the function call node at the cursor position
        let root = tree.root_node();
        let node = root.descendant_for_byte_range(byte_offset, byte_offset);

        if let Some(call_node) = node {
            // Find the enclosing function call
            if let Some((func_name, param_index)) = self.find_function_call(call_node, content, byte_offset, lang) {
                // Look up function signature
                if let Some(signature) = self.find_function_signature(&tree, content, &func_name, lang) {
                    return Ok(Some(SignatureHelp {
                        signatures: vec![signature],
                        active_signature: Some(0),
                        active_parameter: Some(param_index as u32),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Find the function call that contains the cursor position
    fn find_function_call(
        &self,
        mut node: tree_sitter::Node,
        content: &str,
        byte_offset: usize,
        lang: &str,
    ) -> Option<(String, usize)> {
        // Traverse up the tree to find a function call node
        loop {
            match lang {
                "python" => {
                    if node.kind() == "call" {
                        // Get function name from the first child
                        if let Some(func_node) = node.child_by_field_name("function") {
                            let func_name = &content[func_node.start_byte()..func_node.end_byte()];

                            // Find which parameter we're in
                            if let Some(args_node) = node.child_by_field_name("arguments") {
                                let param_index = self.get_parameter_index(args_node, byte_offset, content);
                                return Some((func_name.to_string(), param_index));
                            }
                        }
                    }
                }
                "javascript" | "typescript" | "tsx" => {
                    if node.kind() == "call_expression" {
                        // Get function name
                        if let Some(func_node) = node.child_by_field_name("function") {
                            let func_name = &content[func_node.start_byte()..func_node.end_byte()];

                            // Find which parameter we're in
                            if let Some(args_node) = node.child_by_field_name("arguments") {
                                let param_index = self.get_parameter_index(args_node, byte_offset, content);
                                return Some((func_name.to_string(), param_index));
                            }
                        }
                    }
                }
                "rust" => {
                    if node.kind() == "call_expression" {
                        // Get function name
                        if let Some(func_node) = node.child_by_field_name("function") {
                            let func_name = &content[func_node.start_byte()..func_node.end_byte()];

                            // Find which parameter we're in
                            if let Some(args_node) = node.child_by_field_name("arguments") {
                                let param_index = self.get_parameter_index(args_node, byte_offset, content);
                                return Some((func_name.to_string(), param_index));
                            }
                        }
                    }
                }
                _ => {}
            }

            if let Some(parent) = node.parent() {
                node = parent;
            } else {
                break;
            }
        }

        None
    }

    /// Get the parameter index (which parameter the cursor is on)
    fn get_parameter_index(
        &self,
        args_node: tree_sitter::Node,
        byte_offset: usize,
        _content: &str,
    ) -> usize {
        let mut param_index = 0;
        let mut cursor = args_node.walk();

        // Count commas before the cursor position
        for child in args_node.children(&mut cursor) {
            if child.start_byte() >= byte_offset {
                break;
            }

            if child.kind() == "," {
                param_index += 1;
            }
        }

        param_index
    }

    /// Find the function signature definition
    fn find_function_signature(
        &self,
        tree: &std::sync::Arc<tree_sitter::Tree>,
        content: &str,
        func_name: &str,
        lang: &str,
    ) -> Option<SignatureInformation> {
        let root = tree.root_node();
        let mut cursor = root.walk();

        // Search for function definitions
        match lang {
            "python" => {
                for node in root.children(&mut cursor) {
                    if node.kind() == "function_definition" {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            let name = &content[name_node.start_byte()..name_node.end_byte()];

                            if name == func_name {
                                return self.extract_python_signature(node, content);
                            }
                        }
                    }
                }
            }
            "javascript" | "typescript" | "tsx" => {
                for node in root.children(&mut cursor) {
                    if node.kind() == "function_declaration" || node.kind() == "function" {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            let name = &content[name_node.start_byte()..name_node.end_byte()];

                            if name == func_name {
                                return self.extract_js_signature(node, content);
                            }
                        }
                    }
                }
            }
            "rust" => {
                for node in root.children(&mut cursor) {
                    if node.kind() == "function_item" {
                        if let Some(name_node) = node.child_by_field_name("name") {
                            let name = &content[name_node.start_byte()..name_node.end_byte()];

                            if name == func_name {
                                return self.extract_rust_signature(node, content);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        None
    }

    /// Extract Python function signature
    fn extract_python_signature(
        &self,
        node: tree_sitter::Node,
        content: &str,
    ) -> Option<SignatureInformation> {
        let name_node = node.child_by_field_name("name")?;
        let params_node = node.child_by_field_name("parameters")?;

        let func_name = &content[name_node.start_byte()..name_node.end_byte()];
        let params_text = &content[params_node.start_byte()..params_node.end_byte()];

        // Build full signature
        let label = format!("{}{}", func_name, params_text);

        // Extract parameter information
        let mut parameters = Vec::new();
        let mut cursor = params_node.walk();

        for child in params_node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "typed_parameter" {
                let param_text = &content[child.start_byte()..child.end_byte()];
                parameters.push(ParameterInformation {
                    label: ParameterLabel::Simple(param_text.to_string()),
                    documentation: None,
                });
            }
        }

        Some(SignatureInformation {
            label,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: None,
        })
    }

    /// Extract JavaScript/TypeScript function signature
    fn extract_js_signature(
        &self,
        node: tree_sitter::Node,
        content: &str,
    ) -> Option<SignatureInformation> {
        let name_node = node.child_by_field_name("name")?;
        let params_node = node.child_by_field_name("parameters")?;

        let func_name = &content[name_node.start_byte()..name_node.end_byte()];
        let params_text = &content[params_node.start_byte()..params_node.end_byte()];

        // Build full signature
        let label = format!("{}{}", func_name, params_text);

        // Extract parameter information
        let mut parameters = Vec::new();
        let mut cursor = params_node.walk();

        for child in params_node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "required_parameter" || child.kind() == "optional_parameter" {
                let param_text = &content[child.start_byte()..child.end_byte()];
                parameters.push(ParameterInformation {
                    label: ParameterLabel::Simple(param_text.to_string()),
                    documentation: None,
                });
            }
        }

        Some(SignatureInformation {
            label,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: None,
        })
    }

    /// Extract Rust function signature
    fn extract_rust_signature(
        &self,
        node: tree_sitter::Node,
        content: &str,
    ) -> Option<SignatureInformation> {
        let name_node = node.child_by_field_name("name")?;
        let params_node = node.child_by_field_name("parameters")?;

        let func_name = &content[name_node.start_byte()..name_node.end_byte()];
        let params_text = &content[params_node.start_byte()..params_node.end_byte()];

        // Build full signature
        let label = format!("{}{}", func_name, params_text);

        // Extract parameter information
        let mut parameters = Vec::new();
        let mut cursor = params_node.walk();

        for child in params_node.children(&mut cursor) {
            if child.kind() == "parameter" {
                let param_text = &content[child.start_byte()..child.end_byte()];
                parameters.push(ParameterInformation {
                    label: ParameterLabel::Simple(param_text.to_string()),
                    documentation: None,
                });
            }
        }

        Some(SignatureInformation {
            label,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: None,
        })
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

impl Default for SignatureHelpProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_function_signature() {
        let provider = SignatureHelpProvider::new();
        let content = r#"
def calculate_sum(a, b, c):
    return a + b + c

result = calculate_sum(1, 2, 3)
"#;

        // Position inside the function call (on second argument)
        let position = Position { line: 4, character: 25 };
        let result = provider.get_signature_help(content, position, "python").unwrap();

        assert!(result.is_some());
        let sig_help = result.unwrap();
        assert_eq!(sig_help.signatures.len(), 1);
        assert_eq!(sig_help.signatures[0].label, "calculate_sum(a, b, c)");
        // Since we're at the start of the second argument, should be parameter index 1
        assert_eq!(sig_help.active_parameter, Some(1)); // Should be on parameter 'b'
    }

    #[test]
    fn test_javascript_function_signature() {
        let provider = SignatureHelpProvider::new();
        let content = r#"
function add(x, y) {
    return x + y;
}

const result = add(5, 10);
"#;

        // Position inside the function call (on second argument)
        let position = Position { line: 5, character: 21 };
        let result = provider.get_signature_help(content, position, "javascript").unwrap();

        assert!(result.is_some());
        let sig_help = result.unwrap();
        assert_eq!(sig_help.signatures.len(), 1);
        assert_eq!(sig_help.signatures[0].label, "add(x, y)");
        // Since we're on the second argument, should be parameter index 1
        assert_eq!(sig_help.active_parameter, Some(1)); // Should be on parameter 'y'
    }

    #[test]
    fn test_rust_function_signature() {
        let provider = SignatureHelpProvider::new();
        let content = r#"
fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

fn main() {
    let result = multiply(3, 4);
}
"#;

        // Position inside the function call (after first argument)
        let position = Position { line: 6, character: 28 };
        let result = provider.get_signature_help(content, position, "rust").unwrap();

        assert!(result.is_some());
        let sig_help = result.unwrap();
        assert_eq!(sig_help.signatures.len(), 1);
        assert_eq!(sig_help.signatures[0].label, "multiply(a: i32, b: i32)");
        assert_eq!(sig_help.active_parameter, Some(1)); // Should be on parameter 'b'
    }

    #[test]
    fn test_no_signature_outside_call() {
        let provider = SignatureHelpProvider::new();
        let content = r#"
def test():
    pass

x = 5
"#;

        // Position outside any function call
        let position = Position { line: 4, character: 5 };
        let result = provider.get_signature_help(content, position, "python").unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_parameter_index_calculation() {
        let provider = SignatureHelpProvider::new();
        let content = r#"
def func(a, b, c, d):
    pass

func(1, 2, 3, 4)
"#;

        // Test different positions
        let positions = vec![
            (Position { line: 4, character: 6 }, 0),  // On 'a'
            (Position { line: 4, character: 9 }, 1),  // On 'b'
            (Position { line: 4, character: 12 }, 2), // On 'c'
            (Position { line: 4, character: 15 }, 3), // On 'd'
        ];

        for (pos, expected_param) in positions {
            let result = provider.get_signature_help(content, pos, "python").unwrap();
            if let Some(sig_help) = result {
                assert_eq!(sig_help.active_parameter, Some(expected_param));
            }
        }
    }

    #[test]
    fn test_position_to_byte_conversion() {
        let provider = SignatureHelpProvider::new();
        let source = "hello\nworld\n";

        assert_eq!(provider.position_to_byte(source, Position { line: 0, character: 0 }), 0);
        assert_eq!(provider.position_to_byte(source, Position { line: 0, character: 5 }), 5);
        assert_eq!(provider.position_to_byte(source, Position { line: 1, character: 0 }), 6);
        assert_eq!(provider.position_to_byte(source, Position { line: 1, character: 5 }), 11);
    }
}
