//! Code Lens Provider
//!
//! Provides inline actionable information above symbols:
//! - Reference counting (e.g., "5 references")
//! - Run test / Debug test buttons
//! - Custom commands based on symbol type

use anyhow::Result;
use tower_lsp::lsp_types::*;
use crate::tree_sitter::TreeSitterParser;

/// Code lens provider
#[derive(Debug)]
pub struct CodeLensProvider {
    /// Show reference count above symbols
    show_references: bool,
    /// Show test run buttons
    show_test_actions: bool,
}

impl Default for CodeLensProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeLensProvider {
    pub fn new() -> Self {
        Self {
            show_references: true,
            show_test_actions: true,
        }
    }

    /// Get code lenses for a document
    pub fn get_code_lenses(
        &self,
        content: &str,
        lang: &str,
    ) -> Result<Vec<CodeLens>> {
        let mut parser = TreeSitterParser::new()?;
        if parser.set_language(lang).is_err() {
            return Ok(Vec::new());
        }

        let tree = parser.parse(content, "temp")?;
        let root = tree.root_node();

        let mut lenses = Vec::new();

        // Collect lenses for functions, classes, and test functions
        self.collect_lenses_recursive(root, content, lang, &mut lenses)?;

        Ok(lenses)
    }

    /// Recursively collect code lenses from tree
    fn collect_lenses_recursive(
        &self,
        node: tree_sitter::Node,
        content: &str,
        lang: &str,
        lenses: &mut Vec<CodeLens>,
    ) -> Result<()> {
        // Check if this node should have a code lens
        match lang {
            "python" => self.collect_python_lenses(node, content, lenses)?,
            "javascript" | "typescript" | "tsx" | "jsx" => {
                self.collect_js_lenses(node, content, lenses)?
            }
            "rust" => self.collect_rust_lenses(node, content, lenses)?,
            _ => {}
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_lenses_recursive(child, content, lang, lenses)?;
        }

        Ok(())
    }

    /// Collect code lenses for Python
    fn collect_python_lenses(
        &self,
        node: tree_sitter::Node,
        content: &str,
        lenses: &mut Vec<CodeLens>,
    ) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let func_name = &content[name_node.start_byte()..name_node.end_byte()];
                    let range = self.node_to_range(node, content);

                    // Check if it's a test function
                    if func_name.starts_with("test_") || func_name.starts_with("Test") {
                        if self.show_test_actions {
                            // Add "Run test" lens
                            lenses.push(CodeLens {
                                range,
                                command: Some(Command {
                                    title: "▶ Run test".to_string(),
                                    command: "python.runTest".to_string(),
                                    arguments: Some(vec![
                                        serde_json::Value::String(func_name.to_string())
                                    ]),
                                }),
                                data: None,
                            });

                            // Add "Debug test" lens
                            lenses.push(CodeLens {
                                range,
                                command: Some(Command {
                                    title: "🐛 Debug test".to_string(),
                                    command: "python.debugTest".to_string(),
                                    arguments: Some(vec![
                                        serde_json::Value::String(func_name.to_string())
                                    ]),
                                }),
                                data: None,
                            });
                        }
                    }

                    // Add reference count lens
                    if self.show_references {
                        lenses.push(CodeLens {
                            range,
                            command: Some(Command {
                                title: "0 references".to_string(),
                                command: "editor.action.showReferences".to_string(),
                                arguments: Some(vec![
                                    serde_json::Value::String(func_name.to_string())
                                ]),
                            }),
                            data: Some(serde_json::json!({
                                "type": "references",
                                "symbol": func_name
                            })),
                        });
                    }
                }
            }
            "class_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let class_name = &content[name_node.start_byte()..name_node.end_byte()];
                    let range = self.node_to_range(node, content);

                    // Add reference count lens
                    if self.show_references {
                        lenses.push(CodeLens {
                            range,
                            command: Some(Command {
                                title: "0 references".to_string(),
                                command: "editor.action.showReferences".to_string(),
                                arguments: Some(vec![
                                    serde_json::Value::String(class_name.to_string())
                                ]),
                            }),
                            data: Some(serde_json::json!({
                                "type": "references",
                                "symbol": class_name
                            })),
                        });
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Collect code lenses for JavaScript/TypeScript
    fn collect_js_lenses(
        &self,
        node: tree_sitter::Node,
        content: &str,
        lenses: &mut Vec<CodeLens>,
    ) -> Result<()> {
        match node.kind() {
            "function_declaration" | "method_definition" | "arrow_function" => {
                // Try to get function name
                let func_name = if let Some(name_node) = node.child_by_field_name("name") {
                    Some(&content[name_node.start_byte()..name_node.end_byte()])
                } else {
                    None
                };

                if let Some(name) = func_name {
                    let range = self.node_to_range(node, content);

                    // Check if it's a test function (jest, mocha, vitest patterns)
                    let is_test = name.starts_with("test")
                        || name.starts_with("it")
                        || name.starts_with("describe")
                        || content[node.start_byte()..node.end_byte()].contains("test(")
                        || content[node.start_byte()..node.end_byte()].contains("it(");

                    if is_test && self.show_test_actions {
                        // Add "Run test" lens
                        lenses.push(CodeLens {
                            range,
                            command: Some(Command {
                                title: "▶ Run test".to_string(),
                                command: "javascript.runTest".to_string(),
                                arguments: Some(vec![
                                    serde_json::Value::String(name.to_string())
                                ]),
                            }),
                            data: None,
                        });

                        // Add "Debug test" lens
                        lenses.push(CodeLens {
                            range,
                            command: Some(Command {
                                title: "🐛 Debug test".to_string(),
                                command: "javascript.debugTest".to_string(),
                                arguments: Some(vec![
                                    serde_json::Value::String(name.to_string())
                                ]),
                            }),
                            data: None,
                        });
                    }

                    // Add reference count lens
                    if self.show_references {
                        lenses.push(CodeLens {
                            range,
                            command: Some(Command {
                                title: "0 references".to_string(),
                                command: "editor.action.showReferences".to_string(),
                                arguments: Some(vec![
                                    serde_json::Value::String(name.to_string())
                                ]),
                            }),
                            data: Some(serde_json::json!({
                                "type": "references",
                                "symbol": name
                            })),
                        });
                    }
                }
            }
            "class_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let class_name = &content[name_node.start_byte()..name_node.end_byte()];
                    let range = self.node_to_range(node, content);

                    // Add reference count lens
                    if self.show_references {
                        lenses.push(CodeLens {
                            range,
                            command: Some(Command {
                                title: "0 references".to_string(),
                                command: "editor.action.showReferences".to_string(),
                                arguments: Some(vec![
                                    serde_json::Value::String(class_name.to_string())
                                ]),
                            }),
                            data: Some(serde_json::json!({
                                "type": "references",
                                "symbol": class_name
                            })),
                        });
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Collect code lenses for Rust
    fn collect_rust_lenses(
        &self,
        node: tree_sitter::Node,
        content: &str,
        lenses: &mut Vec<CodeLens>,
    ) -> Result<()> {
        match node.kind() {
            "function_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let func_name = &content[name_node.start_byte()..name_node.end_byte()];
                    let range = self.node_to_range(node, content);

                    // Check for test attributes before the function
                    // Look backwards from function start to find attributes
                    let start = node.start_byte().saturating_sub(100); // Look back up to 100 bytes
                    let context = &content[start..node.end_byte()];

                    // Check if it's a test function
                    let is_test = context.contains("#[test]")
                        || context.contains("#[tokio::test]")
                        || context.contains("#[cfg(test)]");

                    if is_test && self.show_test_actions {
                        // Add "Run test" lens
                        lenses.push(CodeLens {
                            range,
                            command: Some(Command {
                                title: "▶ Run test".to_string(),
                                command: "rust.runTest".to_string(),
                                arguments: Some(vec![
                                    serde_json::Value::String(func_name.to_string())
                                ]),
                            }),
                            data: None,
                        });

                        // Add "Debug test" lens
                        lenses.push(CodeLens {
                            range,
                            command: Some(Command {
                                title: "🐛 Debug test".to_string(),
                                command: "rust.debugTest".to_string(),
                                arguments: Some(vec![
                                    serde_json::Value::String(func_name.to_string())
                                ]),
                            }),
                            data: None,
                        });
                    }

                    // Add reference count lens
                    if self.show_references {
                        lenses.push(CodeLens {
                            range,
                            command: Some(Command {
                                title: "0 references".to_string(),
                                command: "editor.action.showReferences".to_string(),
                                arguments: Some(vec![
                                    serde_json::Value::String(func_name.to_string())
                                ]),
                            }),
                            data: Some(serde_json::json!({
                                "type": "references",
                                "symbol": func_name
                            })),
                        });
                    }
                }
            }
            "struct_item" | "enum_item" | "trait_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let type_name = &content[name_node.start_byte()..name_node.end_byte()];
                    let range = self.node_to_range(node, content);

                    // Add reference count lens
                    if self.show_references {
                        lenses.push(CodeLens {
                            range,
                            command: Some(Command {
                                title: "0 references".to_string(),
                                command: "editor.action.showReferences".to_string(),
                                arguments: Some(vec![
                                    serde_json::Value::String(type_name.to_string())
                                ]),
                            }),
                            data: Some(serde_json::json!({
                                "type": "references",
                                "symbol": type_name
                            })),
                        });
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Convert tree-sitter node to LSP Range
    fn node_to_range(&self, node: tree_sitter::Node, content: &str) -> Range {
        let start = self.byte_to_position(content, node.start_byte());
        let end = self.byte_to_position(content, node.end_byte());
        Range { start, end }
    }

    /// Convert byte offset to LSP Position
    fn byte_to_position(&self, content: &str, byte_offset: usize) -> Position {
        let mut line = 0;
        let mut character = 0;

        for (i, ch) in content.char_indices() {
            if i >= byte_offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }

        Position {
            line: line as u32,
            character: character as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_function_lens() {
        let provider = CodeLensProvider::new();
        let content = r#"
def add(a, b):
    return a + b

def multiply(x, y):
    return x * y
"#;

        let lenses = provider.get_code_lenses(content, "python").unwrap();

        // Should have reference count lens for each function
        assert!(lenses.len() >= 2);
        assert!(lenses.iter().any(|l| l.command.as_ref().unwrap().title.contains("references")));
    }

    #[test]
    fn test_python_test_function_lens() {
        let provider = CodeLensProvider::new();
        let content = r#"
def test_addition():
    assert add(2, 3) == 5

def test_multiplication():
    assert multiply(2, 3) == 6
"#;

        let lenses = provider.get_code_lenses(content, "python").unwrap();

        // Should have run/debug lenses for test functions
        assert!(lenses.iter().any(|l| l.command.as_ref().unwrap().title.contains("Run test")));
        assert!(lenses.iter().any(|l| l.command.as_ref().unwrap().title.contains("Debug test")));
    }

    #[test]
    fn test_python_class_lens() {
        let provider = CodeLensProvider::new();
        let content = r#"
class Calculator:
    def add(self, a, b):
        return a + b
"#;

        let lenses = provider.get_code_lenses(content, "python").unwrap();

        // Should have reference lens for class
        assert!(lenses.len() >= 1);
        assert!(lenses.iter().any(|l| {
            if let Some(data) = &l.data {
                data.get("type") == Some(&serde_json::Value::String("references".to_string()))
            } else {
                false
            }
        }));
    }

    #[test]
    fn test_javascript_function_lens() {
        let provider = CodeLensProvider::new();
        let content = r#"
function add(a, b) {
    return a + b;
}

const multiply = (x, y) => x * y;
"#;

        let lenses = provider.get_code_lenses(content, "javascript").unwrap();

        // Should have reference count lenses
        assert!(lenses.len() >= 1);
        assert!(lenses.iter().any(|l| l.command.as_ref().unwrap().title.contains("references")));
    }

    #[test]
    fn test_javascript_class_lens() {
        let provider = CodeLensProvider::new();
        let content = r#"
class Calculator {
    add(a, b) {
        return a + b;
    }
}
"#;

        let lenses = provider.get_code_lenses(content, "javascript").unwrap();

        // Should have reference lens for class and method
        assert!(lenses.len() >= 1);
    }

    #[test]
    fn test_rust_function_lens() {
        let provider = CodeLensProvider::new();
        let content = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn multiply(x: i32, y: i32) -> i32 {
    x * y
}
"#;

        let lenses = provider.get_code_lenses(content, "rust").unwrap();

        // Should have reference count lenses
        assert!(lenses.len() >= 2);
        assert!(lenses.iter().any(|l| l.command.as_ref().unwrap().title.contains("references")));
    }

    #[test]
    fn test_rust_test_function_lens() {
        let provider = CodeLensProvider::new();
        let content = r#"
#[test]
fn test_addition() {
    assert_eq!(add(2, 3), 5);
}

#[tokio::test]
async fn test_async_operation() {
    assert!(true);
}
"#;

        let lenses = provider.get_code_lenses(content, "rust").unwrap();

        // Should have run/debug lenses for test functions
        assert!(lenses.iter().any(|l| l.command.as_ref().unwrap().title.contains("Run test")));
        assert!(lenses.iter().any(|l| l.command.as_ref().unwrap().title.contains("Debug test")));
    }

    #[test]
    fn test_rust_struct_lens() {
        let provider = CodeLensProvider::new();
        let content = r#"
pub struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new() -> Self {
        Self { value: 0 }
    }
}
"#;

        let lenses = provider.get_code_lenses(content, "rust").unwrap();

        // Should have reference lens for struct
        assert!(lenses.len() >= 1);
        assert!(lenses.iter().any(|l| {
            if let Some(data) = &l.data {
                data.get("symbol") == Some(&serde_json::Value::String("Calculator".to_string()))
            } else {
                false
            }
        }));
    }

    #[test]
    fn test_unsupported_language() {
        let provider = CodeLensProvider::new();
        let content = "some random content";

        let lenses = provider.get_code_lenses(content, "unknown").unwrap();

        // Should return empty for unsupported languages
        assert!(lenses.is_empty());
    }

    #[test]
    fn test_empty_content() {
        let provider = CodeLensProvider::new();
        let content = "";

        let lenses = provider.get_code_lenses(content, "python").unwrap();

        // Should handle empty content gracefully
        assert!(lenses.is_empty());
    }

    #[test]
    fn test_code_lens_commands() {
        let provider = CodeLensProvider::new();
        let content = r#"
def test_something():
    pass
"#;

        let lenses = provider.get_code_lenses(content, "python").unwrap();

        // Verify command structure
        for lens in lenses {
            if let Some(cmd) = &lens.command {
                assert!(!cmd.title.is_empty());
                assert!(!cmd.command.is_empty());
            }
        }
    }

    #[test]
    fn test_position_conversion() {
        let provider = CodeLensProvider::new();
        let content = "line1\nline2\nline3";

        let pos1 = provider.byte_to_position(content, 0);
        assert_eq!(pos1.line, 0);
        assert_eq!(pos1.character, 0);

        let pos2 = provider.byte_to_position(content, 6);
        assert_eq!(pos2.line, 1);
        assert_eq!(pos2.character, 0);
    }
}
