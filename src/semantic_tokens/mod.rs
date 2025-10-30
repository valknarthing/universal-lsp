//! Semantic Tokens Module
//!
//! Provides enhanced syntax highlighting through semantic token classification

use anyhow::Result;
use tower_lsp::lsp_types::*;
use crate::tree_sitter::TreeSitterParser;

/// Semantic token types (standard LSP token types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    Namespace = 0,
    Type = 1,
    Class = 2,
    Enum = 3,
    Interface = 4,
    Struct = 5,
    TypeParameter = 6,
    Parameter = 7,
    Variable = 8,
    Property = 9,
    EnumMember = 10,
    Event = 11,
    Function = 12,
    Method = 13,
    Macro = 14,
    Keyword = 15,
    Modifier = 16,
    Comment = 17,
    String = 18,
    Number = 19,
    Regexp = 20,
    Operator = 21,
}

impl TokenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenType::Namespace => "namespace",
            TokenType::Type => "type",
            TokenType::Class => "class",
            TokenType::Enum => "enum",
            TokenType::Interface => "interface",
            TokenType::Struct => "struct",
            TokenType::TypeParameter => "typeParameter",
            TokenType::Parameter => "parameter",
            TokenType::Variable => "variable",
            TokenType::Property => "property",
            TokenType::EnumMember => "enumMember",
            TokenType::Event => "event",
            TokenType::Function => "function",
            TokenType::Method => "method",
            TokenType::Macro => "macro",
            TokenType::Keyword => "keyword",
            TokenType::Modifier => "modifier",
            TokenType::Comment => "comment",
            TokenType::String => "string",
            TokenType::Number => "number",
            TokenType::Regexp => "regexp",
            TokenType::Operator => "operator",
        }
    }

    pub fn legend() -> Vec<SemanticTokenType> {
        vec![
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::TYPE,
            SemanticTokenType::CLASS,
            SemanticTokenType::ENUM,
            SemanticTokenType::INTERFACE,
            SemanticTokenType::STRUCT,
            SemanticTokenType::TYPE_PARAMETER,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::EVENT,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::METHOD,
            SemanticTokenType::MACRO,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::MODIFIER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::REGEXP,
            SemanticTokenType::OPERATOR,
        ]
    }
}

/// Semantic token modifiers (bitflags)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenModifier {
    Declaration = 0,
    Definition = 1,
    Readonly = 2,
    Static = 3,
    Deprecated = 4,
    Abstract = 5,
    Async = 6,
    Modification = 7,
    Documentation = 8,
    DefaultLibrary = 9,
}

impl TokenModifier {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenModifier::Declaration => "declaration",
            TokenModifier::Definition => "definition",
            TokenModifier::Readonly => "readonly",
            TokenModifier::Static => "static",
            TokenModifier::Deprecated => "deprecated",
            TokenModifier::Abstract => "abstract",
            TokenModifier::Async => "async",
            TokenModifier::Modification => "modification",
            TokenModifier::Documentation => "documentation",
            TokenModifier::DefaultLibrary => "defaultLibrary",
        }
    }

    pub fn to_bitmask(&self) -> u32 {
        1 << (*self as u32)
    }

    pub fn legend() -> Vec<SemanticTokenModifier> {
        vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::STATIC,
            SemanticTokenModifier::DEPRECATED,
            SemanticTokenModifier::ABSTRACT,
            SemanticTokenModifier::ASYNC,
            SemanticTokenModifier::MODIFICATION,
            SemanticTokenModifier::DOCUMENTATION,
            SemanticTokenModifier::DEFAULT_LIBRARY,
        ]
    }
}

/// A classified token with position and type information
#[derive(Debug, Clone)]
struct ClassifiedToken {
    line: u32,
    start_char: u32,
    length: u32,
    token_type: TokenType,
    modifiers: u32,
}

/// Semantic tokens provider
#[derive(Debug)]
pub struct SemanticTokensProvider {}

impl SemanticTokensProvider {
    pub fn new() -> Self {
        Self {}
    }

    /// Get the legend for semantic tokens
    pub fn legend() -> SemanticTokensLegend {
        SemanticTokensLegend {
            token_types: TokenType::legend(),
            token_modifiers: TokenModifier::legend(),
        }
    }

    /// Get semantic tokens for entire document
    pub fn get_semantic_tokens(
        &self,
        content: &str,
        lang: &str,
    ) -> Result<Option<SemanticTokens>> {
        let tokens = self.classify_tokens(content, lang)?;
        let encoded = self.encode_tokens(tokens);

        Ok(Some(SemanticTokens {
            result_id: None,
            data: encoded,
        }))
    }

    /// Classify all tokens in the document
    fn classify_tokens(&self, content: &str, lang: &str) -> Result<Vec<ClassifiedToken>> {
        let mut parser = TreeSitterParser::new()?;
        if parser.set_language(lang).is_err() {
            return Ok(Vec::new());
        }

        let tree = parser.parse(content, "temp")?;
        let root = tree.root_node();

        let mut tokens = Vec::new();
        let mut cursor = root.walk();

        self.classify_node_recursive(root, content, lang, &mut tokens, &mut cursor)?;

        // Sort tokens by position
        tokens.sort_by(|a, b| {
            if a.line != b.line {
                a.line.cmp(&b.line)
            } else {
                a.start_char.cmp(&b.start_char)
            }
        });

        Ok(tokens)
    }

    /// Recursively classify nodes
    fn classify_node_recursive(
        &self,
        node: tree_sitter::Node,
        content: &str,
        lang: &str,
        tokens: &mut Vec<ClassifiedToken>,
        cursor: &mut tree_sitter::TreeCursor,
    ) -> Result<()> {
        // Classify this node if it's a relevant token
        if let Some(token) = self.classify_node(&node, content, lang) {
            tokens.push(token);
        }

        // Recursively process children
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                self.classify_node_recursive(child, content, lang, tokens, cursor)?;

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }

        Ok(())
    }

    /// Classify a single node
    fn classify_node(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        lang: &str,
    ) -> Option<ClassifiedToken> {
        let kind = node.kind();
        let start_pos = node.start_position();
        let end_pos = node.end_position();

        // Only classify named nodes
        if !node.is_named() {
            return None;
        }

        let (token_type, modifiers) = match lang {
            "python" => self.classify_python_node(node, content, kind)?,
            "javascript" | "typescript" | "tsx" | "jsx" => {
                self.classify_js_node(node, content, kind)?
            }
            "rust" => self.classify_rust_node(node, content, kind)?,
            _ => return None,
        };

        Some(ClassifiedToken {
            line: start_pos.row as u32,
            start_char: start_pos.column as u32,
            length: (end_pos.column - start_pos.column) as u32,
            token_type,
            modifiers,
        })
    }

    /// Classify Python nodes
    fn classify_python_node(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        kind: &str,
    ) -> Option<(TokenType, u32)> {
        match kind {
            "function_definition" => {
                if let Some(_name_node) = node.child_by_field_name("name") {
                    return Some((TokenType::Function, TokenModifier::Definition.to_bitmask()));
                }
            }
            "class_definition" => {
                if let Some(_name_node) = node.child_by_field_name("name") {
                    return Some((TokenType::Class, TokenModifier::Definition.to_bitmask()));
                }
            }
            "identifier" => {
                let text = &content[node.start_byte()..node.end_byte()];

                // Check if it's a known keyword or builtin
                if self.is_python_keyword(text) {
                    return Some((TokenType::Keyword, 0));
                }

                // Default to variable
                return Some((TokenType::Variable, 0));
            }
            "parameter" => {
                return Some((TokenType::Parameter, 0));
            }
            "comment" => {
                return Some((TokenType::Comment, TokenModifier::Documentation.to_bitmask()));
            }
            "string" | "string_content" => {
                return Some((TokenType::String, 0));
            }
            "integer" | "float" => {
                return Some((TokenType::Number, 0));
            }
            _ => {}
        }

        None
    }

    /// Classify JavaScript/TypeScript nodes
    fn classify_js_node(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        kind: &str,
    ) -> Option<(TokenType, u32)> {
        match kind {
            "function_declaration" | "function" => {
                if let Some(_name_node) = node.child_by_field_name("name") {
                    return Some((TokenType::Function, TokenModifier::Definition.to_bitmask()));
                }
            }
            "class_declaration" | "class" => {
                if let Some(_name_node) = node.child_by_field_name("name") {
                    return Some((TokenType::Class, TokenModifier::Definition.to_bitmask()));
                }
            }
            "method_definition" => {
                return Some((TokenType::Method, TokenModifier::Definition.to_bitmask()));
            }
            "identifier" => {
                let text = &content[node.start_byte()..node.end_byte()];

                if self.is_js_keyword(text) {
                    return Some((TokenType::Keyword, 0));
                }

                return Some((TokenType::Variable, 0));
            }
            "property_identifier" => {
                return Some((TokenType::Property, 0));
            }
            "required_parameter" | "optional_parameter" => {
                return Some((TokenType::Parameter, 0));
            }
            "comment" => {
                return Some((TokenType::Comment, TokenModifier::Documentation.to_bitmask()));
            }
            "string" | "template_string" => {
                return Some((TokenType::String, 0));
            }
            "number" => {
                return Some((TokenType::Number, 0));
            }
            "regex" => {
                return Some((TokenType::Regexp, 0));
            }
            _ => {}
        }

        None
    }

    /// Classify Rust nodes
    fn classify_rust_node(
        &self,
        node: &tree_sitter::Node,
        content: &str,
        kind: &str,
    ) -> Option<(TokenType, u32)> {
        match kind {
            "function_item" => {
                if let Some(_name_node) = node.child_by_field_name("name") {
                    return Some((TokenType::Function, TokenModifier::Definition.to_bitmask()));
                }
            }
            "struct_item" => {
                if let Some(_name_node) = node.child_by_field_name("name") {
                    return Some((TokenType::Struct, TokenModifier::Definition.to_bitmask()));
                }
            }
            "enum_item" => {
                if let Some(_name_node) = node.child_by_field_name("name") {
                    return Some((TokenType::Enum, TokenModifier::Definition.to_bitmask()));
                }
            }
            "trait_item" => {
                return Some((TokenType::Interface, TokenModifier::Definition.to_bitmask()));
            }
            "impl_item" => {
                return Some((TokenType::Class, 0));
            }
            "identifier" => {
                let text = &content[node.start_byte()..node.end_byte()];

                if self.is_rust_keyword(text) {
                    return Some((TokenType::Keyword, 0));
                }

                // Check for mutable binding
                if let Some(parent) = node.parent() {
                    if parent.kind() == "mutable_pattern" {
                        return Some((TokenType::Variable, TokenModifier::Modification.to_bitmask()));
                    }
                }

                return Some((TokenType::Variable, TokenModifier::Readonly.to_bitmask()));
            }
            "parameter" => {
                return Some((TokenType::Parameter, 0));
            }
            "field_identifier" => {
                return Some((TokenType::Property, 0));
            }
            "line_comment" | "block_comment" => {
                return Some((TokenType::Comment, TokenModifier::Documentation.to_bitmask()));
            }
            "string_literal" | "raw_string_literal" | "char_literal" => {
                return Some((TokenType::String, 0));
            }
            "integer_literal" | "float_literal" => {
                return Some((TokenType::Number, 0));
            }
            "macro_invocation" => {
                return Some((TokenType::Macro, 0));
            }
            _ => {}
        }

        None
    }

    /// Encode tokens to LSP format (delta encoding)
    fn encode_tokens(&self, tokens: Vec<ClassifiedToken>) -> Vec<SemanticToken> {
        let mut result = Vec::new();
        let mut prev_line = 0;
        let mut prev_start = 0;

        for token in tokens {
            let delta_line = token.line - prev_line;
            let delta_start = if delta_line == 0 {
                token.start_char - prev_start
            } else {
                token.start_char
            };

            result.push(SemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type: token.token_type as u32,
                token_modifiers_bitset: token.modifiers,
            });

            prev_line = token.line;
            prev_start = token.start_char;
        }

        result
    }

    /// Check if text is a Python keyword
    fn is_python_keyword(&self, text: &str) -> bool {
        matches!(
            text,
            "and" | "as" | "assert" | "async" | "await" | "break" | "class" | "continue"
                | "def" | "del" | "elif" | "else" | "except" | "finally" | "for" | "from"
                | "global" | "if" | "import" | "in" | "is" | "lambda" | "nonlocal" | "not"
                | "or" | "pass" | "raise" | "return" | "try" | "while" | "with" | "yield"
        )
    }

    /// Check if text is a JavaScript keyword
    fn is_js_keyword(&self, text: &str) -> bool {
        matches!(
            text,
            "break" | "case" | "catch" | "class" | "const" | "continue" | "debugger"
                | "default" | "delete" | "do" | "else" | "export" | "extends" | "finally"
                | "for" | "function" | "if" | "import" | "in" | "instanceof" | "let" | "new"
                | "return" | "super" | "switch" | "this" | "throw" | "try" | "typeof" | "var"
                | "void" | "while" | "with" | "yield"
        )
    }

    /// Check if text is a Rust keyword
    fn is_rust_keyword(&self, text: &str) -> bool {
        matches!(
            text,
            "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern"
                | "false" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match"
                | "mod" | "move" | "mut" | "pub" | "ref" | "return" | "self" | "Self"
                | "static" | "struct" | "super" | "trait" | "true" | "type" | "unsafe" | "use"
                | "where" | "while" | "async" | "await" | "dyn"
        )
    }
}

impl Default for SemanticTokensProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_legend() {
        let legend = SemanticTokensProvider::legend();
        assert_eq!(legend.token_types.len(), 22);
        assert_eq!(legend.token_modifiers.len(), 10);
    }

    #[test]
    fn test_python_semantic_tokens() {
        let provider = SemanticTokensProvider::new();
        let content = r#"
def calculate(a, b):
    result = a + b
    return result

class MyClass:
    def __init__(self):
        self.value = 42
"#;

        let result = provider.get_semantic_tokens(content, "python");
        assert!(result.is_ok());

        let tokens = result.unwrap();
        assert!(tokens.is_some());

        let semantic_tokens = tokens.unwrap();
        assert!(!semantic_tokens.data.is_empty());
    }

    #[test]
    fn test_javascript_semantic_tokens() {
        let provider = SemanticTokensProvider::new();
        let content = r#"
function calculate(a, b) {
    const result = a + b;
    return result;
}

class MyClass {
    constructor() {
        this.value = 42;
    }
}
"#;

        let result = provider.get_semantic_tokens(content, "javascript");
        assert!(result.is_ok());

        let tokens = result.unwrap();
        assert!(tokens.is_some());

        let semantic_tokens = tokens.unwrap();
        assert!(!semantic_tokens.data.is_empty());
    }

    #[test]
    fn test_rust_semantic_tokens() {
        let provider = SemanticTokensProvider::new();
        let content = r#"
fn calculate(a: i32, b: i32) -> i32 {
    let result = a + b;
    result
}

struct MyStruct {
    value: i32,
}

impl MyStruct {
    fn new(value: i32) -> Self {
        MyStruct { value }
    }
}
"#;

        let result = provider.get_semantic_tokens(content, "rust");
        assert!(result.is_ok());

        let tokens = result.unwrap();
        assert!(tokens.is_some());

        let semantic_tokens = tokens.unwrap();
        assert!(!semantic_tokens.data.is_empty());
    }

    #[test]
    fn test_token_type_classification() {
        assert_eq!(TokenType::Function.as_str(), "function");
        assert_eq!(TokenType::Class.as_str(), "class");
        assert_eq!(TokenType::Variable.as_str(), "variable");
        assert_eq!(TokenType::Parameter.as_str(), "parameter");
    }

    #[test]
    fn test_token_modifier_bitmask() {
        assert_eq!(TokenModifier::Readonly.to_bitmask(), 1 << 2);
        assert_eq!(TokenModifier::Static.to_bitmask(), 1 << 3);
        assert_eq!(TokenModifier::Definition.to_bitmask(), 1 << 1);
    }

    #[test]
    fn test_python_keywords() {
        let provider = SemanticTokensProvider::new();
        assert!(provider.is_python_keyword("def"));
        assert!(provider.is_python_keyword("class"));
        assert!(provider.is_python_keyword("if"));
        assert!(!provider.is_python_keyword("calculate"));
    }

    #[test]
    fn test_javascript_keywords() {
        let provider = SemanticTokensProvider::new();
        assert!(provider.is_js_keyword("function"));
        assert!(provider.is_js_keyword("const"));
        assert!(provider.is_js_keyword("class"));
        assert!(!provider.is_js_keyword("myVariable"));
    }

    #[test]
    fn test_rust_keywords() {
        let provider = SemanticTokensProvider::new();
        assert!(provider.is_rust_keyword("fn"));
        assert!(provider.is_rust_keyword("struct"));
        assert!(provider.is_rust_keyword("impl"));
        assert!(provider.is_rust_keyword("mut"));
        assert!(!provider.is_rust_keyword("calculate"));
    }

    #[test]
    fn test_empty_content() {
        let provider = SemanticTokensProvider::new();
        let result = provider.get_semantic_tokens("", "python");
        assert!(result.is_ok());
    }

    #[test]
    fn test_unsupported_language() {
        let provider = SemanticTokensProvider::new();
        let result = provider.get_semantic_tokens("some code", "unsupported");
        assert!(result.is_ok());

        let tokens = result.unwrap();
        assert!(tokens.is_some());

        let semantic_tokens = tokens.unwrap();
        assert_eq!(semantic_tokens.data.len(), 0);
    }
}
