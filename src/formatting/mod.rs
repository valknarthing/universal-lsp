//! Code Formatting Module
//!
//! Provides formatting capabilities using tree-sitter and external formatters

use anyhow::Result;
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
    fn try_external_formatter(&self, _content: &str, lang: &str) -> Result<String> {
        match lang {
            "javascript" | "typescript" | "tsx" | "json" => {
                // Would integrate with prettier here
                Err(anyhow::anyhow!("External formatter not configured"))
            }
            "python" => {
                // Would integrate with black/autopep8 here
                Err(anyhow::anyhow!("External formatter not configured"))
            }
            "rust" => {
                // Would integrate with rustfmt here
                Err(anyhow::anyhow!("External formatter not configured"))
            }
            "go" => {
                // Would integrate with gofmt here
                Err(anyhow::anyhow!("External formatter not configured"))
            }
            _ => Err(anyhow::anyhow!("No external formatter for {}", lang)),
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
}
