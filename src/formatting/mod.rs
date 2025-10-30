//! Code Formatting Module
//!
//! Provides formatting capabilities using tree-sitter and external formatters

use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::io::Write;
use tower_lsp::lsp_types::*;
use crate::tree_sitter::TreeSitterParser;

/// Formatting provider for code formatting
#[derive(Debug)]
pub struct FormattingProvider {
    indent_size: usize,
    use_tabs: bool,
}

impl FormattingProvider {
    pub fn new() -> Self {
        Self {
            indent_size: 4,
            use_tabs: false,
        }
    }

    /// Configure formatting settings
    pub fn with_config(mut self, indent_size: usize, use_tabs: bool) -> Self {
        self.indent_size = indent_size;
        self.use_tabs = use_tabs;
        self
    }

    /// Format entire document
    pub fn format_document(
        &self,
        content: &str,
        lang: &str,
        uri: &Url,
    ) -> Result<Vec<TextEdit>> {
        // Try external formatter first
        if let Ok(formatted) = self.try_external_formatter(content, lang) {
            return Ok(vec![TextEdit {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: self.get_end_position(content),
                },
                new_text: formatted,
            }]);
        }

        // Fall back to tree-sitter based formatting
        self.format_with_tree_sitter(content, lang, uri)
    }

    /// Format a specific range
    pub fn format_range(
        &self,
        content: &str,
        range: Range,
        lang: &str,
        _uri: &Url,
    ) -> Result<Vec<TextEdit>> {
        // Extract range text
        let start_byte = self.position_to_byte(content, range.start);
        let end_byte = self.position_to_byte(content, range.end);
        let range_text = &content[start_byte..end_byte];

        // Format just the range
        if let Ok(formatted) = self.try_external_formatter(range_text, lang) {
            return Ok(vec![TextEdit {
                range,
                new_text: formatted,
            }]);
        }

        // Fall back to basic formatting
        self.format_basic(range_text, range)
    }

    /// Try to use external formatter
    fn try_external_formatter(&self, content: &str, lang: &str) -> Result<String> {
        match lang {
            "javascript" | "typescript" | "tsx" | "jsx" => {
                self.format_with_prettier(content, lang)
            }
            "json" => {
                self.format_with_prettier(content, "json")
            }
            "python" => {
                self.format_with_black(content)
            }
            "rust" => {
                self.format_with_rustfmt(content)
            }
            "go" => {
                self.format_with_gofmt(content)
            }
            _ => Err(anyhow::anyhow!("No external formatter for {}", lang)),
        }
    }

    /// Format Python code with black
    fn format_with_black(&self, content: &str) -> Result<String> {
        let mut child = Command::new("black")
            .arg("-")
            .arg("--quiet")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn black. Is it installed? Try: pip install black")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())
                .context("Failed to write to black stdin")?;
        }

        let output = child.wait_with_output()
            .context("Failed to wait for black")?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .context("Black output was not valid UTF-8")
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Black formatting failed: {}", stderr))
        }
    }

    /// Format JavaScript/TypeScript with prettier
    fn format_with_prettier(&self, content: &str, lang: &str) -> Result<String> {
        let parser = match lang {
            "javascript" | "jsx" => "babel",
            "typescript" => "typescript",
            "tsx" => "typescript",
            "json" => "json",
            _ => "babel",
        };

        let mut child = Command::new("prettier")
            .arg("--stdin-filepath")
            .arg(format!("file.{}", self.get_extension(lang)))
            .arg("--parser")
            .arg(parser)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn prettier. Is it installed? Try: npm install -g prettier")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())
                .context("Failed to write to prettier stdin")?;
        }

        let output = child.wait_with_output()
            .context("Failed to wait for prettier")?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .context("Prettier output was not valid UTF-8")
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Prettier formatting failed: {}", stderr))
        }
    }

    /// Format Rust code with rustfmt
    fn format_with_rustfmt(&self, content: &str) -> Result<String> {
        let mut child = Command::new("rustfmt")
            .arg("--emit")
            .arg("stdout")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn rustfmt. Is it installed? Try: rustup component add rustfmt")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())
                .context("Failed to write to rustfmt stdin")?;
        }

        let output = child.wait_with_output()
            .context("Failed to wait for rustfmt")?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .context("Rustfmt output was not valid UTF-8")
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Rustfmt formatting failed: {}", stderr))
        }
    }

    /// Format Go code with gofmt
    fn format_with_gofmt(&self, content: &str) -> Result<String> {
        let mut child = Command::new("gofmt")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn gofmt. Is Go installed?")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())
                .context("Failed to write to gofmt stdin")?;
        }

        let output = child.wait_with_output()
            .context("Failed to wait for gofmt")?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .context("Gofmt output was not valid UTF-8")
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Gofmt formatting failed: {}", stderr))
        }
    }

    /// Get file extension for language
    fn get_extension(&self, lang: &str) -> &str {
        match lang {
            "javascript" => "js",
            "typescript" => "ts",
            "tsx" => "tsx",
            "jsx" => "jsx",
            "json" => "json",
            "python" => "py",
            "rust" => "rs",
            "go" => "go",
            _ => "txt",
        }
    }

    /// Format using tree-sitter
    fn format_with_tree_sitter(
        &self,
        content: &str,
        lang: &str,
        uri: &Url,
    ) -> Result<Vec<TextEdit>> {
        let mut parser = TreeSitterParser::new()?;
        if parser.set_language(lang).is_ok() {
            if let Ok(tree) = parser.parse(content, uri.as_str()) {
                return self.format_tree(&tree, content);
            }
        }

        // Fall back to basic formatting
        let end_pos = self.get_end_position(content);
        self.format_basic(content, Range {
            start: Position { line: 0, character: 0 },
            end: end_pos,
        })
    }

    /// Format based on AST structure
    fn format_tree(&self, tree: &tree_sitter::Tree, source: &str) -> Result<Vec<TextEdit>> {
        let mut edits = Vec::new();
        let root = tree.root_node();

        // Walk the tree and collect formatting edits
        self.format_node_recursive(root, source, 0, &mut edits)?;

        Ok(edits)
    }

    /// Recursively format nodes
    fn format_node_recursive(
        &self,
        node: tree_sitter::Node,
        source: &str,
        depth: usize,
        edits: &mut Vec<TextEdit>,
    ) -> Result<()> {
        // Check if node needs indentation fix
        if node.start_position().column != depth * self.indent_size {
            let indent = if self.use_tabs {
                "\t".repeat(depth)
            } else {
                " ".repeat(depth * self.indent_size)
            };

            let start_pos = Position {
                line: node.start_position().row as u32,
                character: 0,
            };
            let end_pos = Position {
                line: node.start_position().row as u32,
                character: node.start_position().column as u32,
            };

            edits.push(TextEdit {
                range: Range {
                    start: start_pos,
                    end: end_pos,
                },
                new_text: indent,
            });
        }

        // Process children
        let mut child_depth = depth;
        if self.should_increase_indent(&node) {
            child_depth += 1;
        }

        for child in node.children(&mut node.walk()) {
            self.format_node_recursive(child, source, child_depth, edits)?;
        }

        Ok(())
    }

    /// Check if node should increase indentation for children
    fn should_increase_indent(&self, node: &tree_sitter::Node) -> bool {
        matches!(
            node.kind(),
            "block" | "statement_block" | "function_definition" | "class_definition" |
            "function_declaration" | "class_declaration" | "object" | "array"
        )
    }

    /// Basic formatting (whitespace normalization)
    fn format_basic(&self, content: &str, range: Range) -> Result<Vec<TextEdit>> {
        let mut formatted = String::new();
        let mut in_whitespace = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                if in_whitespace {
                    formatted.push('\n');
                }
                formatted.push_str(trimmed);
                formatted.push('\n');
                in_whitespace = false;
            } else {
                in_whitespace = true;
            }
        }

        Ok(vec![TextEdit {
            range,
            new_text: formatted,
        }])
    }

    /// Get end position of content
    fn get_end_position(&self, content: &str) -> Position {
        let lines: Vec<&str> = content.lines().collect();
        let line = lines.len().saturating_sub(1) as u32;
        let character = lines.last().map(|l| l.len()).unwrap_or(0) as u32;
        Position { line, character }
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
}

impl Default for FormattingProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_formatting() {
        let formatter = FormattingProvider::new();
        let content = "fn main()   {   \n\n\n    println!(\"hello\");    \n\n}";
        let uri = Url::parse("file:///test.rs").unwrap();

        let edits = formatter.format_document(content, "rust", &uri);
        assert!(edits.is_ok());
    }

    #[test]
    fn test_get_end_position() {
        let formatter = FormattingProvider::new();
        let content = "line1\nline2\nline3";
        let pos = formatter.get_end_position(content);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 5);
    }

    #[test]
    fn test_python_formatting_with_black() {
        let formatter = FormattingProvider::new();
        let content = "def   hello(  ):  print('world')";

        // Try formatting with black
        let result = formatter.format_with_black(content);

        // If black is installed, should succeed and format correctly
        // If not installed, should error gracefully
        match result {
            Ok(formatted) => {
                assert!(formatted.contains("def hello():"));
                assert!(formatted.contains("print"));
            }
            Err(e) => {
                // Black not installed - that's okay for testing
                let err_msg = e.to_string();
                assert!(err_msg.contains("black") || err_msg.contains("Failed to spawn"));
            }
        }
    }

    #[test]
    fn test_javascript_formatting_with_prettier() {
        let formatter = FormattingProvider::new();
        let content = "function   hello(  ) {  console.log('world')  }";

        let result = formatter.format_with_prettier(content, "javascript");

        match result {
            Ok(formatted) => {
                // Prettier should clean up the whitespace
                assert!(formatted.contains("function hello()"));
            }
            Err(e) => {
                // Prettier not installed - that's okay for testing
                let err_msg = e.to_string();
                assert!(err_msg.contains("prettier") || err_msg.contains("Failed to spawn"));
            }
        }
    }

    #[test]
    fn test_rust_formatting_with_rustfmt() {
        let formatter = FormattingProvider::new();
        let content = "fn   main(  ) {  println!(\"hello\");  }";

        let result = formatter.format_with_rustfmt(content);

        match result {
            Ok(formatted) => {
                // Rustfmt should clean up the whitespace
                assert!(formatted.contains("fn main()"));
                assert!(formatted.contains("println!"));
            }
            Err(e) => {
                // Rustfmt not installed - that's okay for testing
                let err_msg = e.to_string();
                assert!(err_msg.contains("rustfmt") || err_msg.contains("Failed to spawn"));
            }
        }
    }

    #[test]
    fn test_go_formatting_with_gofmt() {
        let formatter = FormattingProvider::new();
        let content = "package main\nfunc   main(  ) {  println(\"hello\")  }";

        let result = formatter.format_with_gofmt(content);

        match result {
            Ok(formatted) => {
                // Gofmt should clean up the whitespace
                assert!(formatted.contains("func main()"));
            }
            Err(e) => {
                // Gofmt not installed - that's okay for testing
                let err_msg = e.to_string();
                assert!(err_msg.contains("gofmt") || err_msg.contains("Failed to spawn"));
            }
        }
    }

    #[test]
    fn test_format_range() {
        let formatter = FormattingProvider::new();
        let content = "def hello():\n    print('world')\n\ndef goodbye():\n    print('bye')";
        let uri = Url::parse("file:///test.py").unwrap();

        // Format just the first function
        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 1, character: 20 },
        };

        let edits = formatter.format_range(content, range, "python", &uri);
        assert!(edits.is_ok());

        let edits = edits.unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].range, range);
    }

    #[test]
    fn test_format_document_fallback() {
        let formatter = FormattingProvider::new();
        // Use an unsupported language to test fallback
        let content = "some random text\n  with   weird spacing";
        let uri = Url::parse("file:///test.txt").unwrap();

        let edits = formatter.format_document(content, "unsupported", &uri);
        assert!(edits.is_ok());
    }

    #[test]
    fn test_position_to_byte() {
        let formatter = FormattingProvider::new();
        let content = "hello\nworld\ntest";

        // Position at start
        assert_eq!(formatter.position_to_byte(content, Position { line: 0, character: 0 }), 0);

        // Position at end of first line
        assert_eq!(formatter.position_to_byte(content, Position { line: 0, character: 5 }), 5);

        // Position at start of second line (after newline)
        assert_eq!(formatter.position_to_byte(content, Position { line: 1, character: 0 }), 6);

        // Position in middle of second line
        assert_eq!(formatter.position_to_byte(content, Position { line: 1, character: 3 }), 9);
    }

    #[test]
    fn test_get_extension() {
        let formatter = FormattingProvider::new();

        assert_eq!(formatter.get_extension("javascript"), "js");
        assert_eq!(formatter.get_extension("typescript"), "ts");
        assert_eq!(formatter.get_extension("tsx"), "tsx");
        assert_eq!(formatter.get_extension("jsx"), "jsx");
        assert_eq!(formatter.get_extension("python"), "py");
        assert_eq!(formatter.get_extension("rust"), "rs");
        assert_eq!(formatter.get_extension("go"), "go");
        assert_eq!(formatter.get_extension("json"), "json");
        assert_eq!(formatter.get_extension("unknown"), "txt");
    }

    #[test]
    fn test_format_with_config() {
        let formatter = FormattingProvider::new()
            .with_config(2, true);

        assert_eq!(formatter.indent_size, 2);
        assert_eq!(formatter.use_tabs, true);
    }

    #[test]
    fn test_empty_content_formatting() {
        let formatter = FormattingProvider::new();
        let content = "";
        let uri = Url::parse("file:///test.rs").unwrap();

        let edits = formatter.format_document(content, "rust", &uri);
        assert!(edits.is_ok());
    }

    #[test]
    fn test_multiline_python_formatting() {
        let formatter = FormattingProvider::new();
        let content = r#"
def calculate(a,b,c):
    result=a+b+c
    return result

class MyClass:
    def __init__(self):
        self.value=42
"#;
        let uri = Url::parse("file:///test.py").unwrap();

        let edits = formatter.format_document(content, "python", &uri);
        assert!(edits.is_ok());
    }

    #[test]
    fn test_multiline_javascript_formatting() {
        let formatter = FormattingProvider::new();
        let content = r#"
function calculate(a,b,c){
    const result=a+b+c;
    return result;
}

class MyClass{
    constructor(){
        this.value=42;
    }
}
"#;
        let uri = Url::parse("file:///test.js").unwrap();

        let edits = formatter.format_document(content, "javascript", &uri);
        assert!(edits.is_ok());
    }
}
