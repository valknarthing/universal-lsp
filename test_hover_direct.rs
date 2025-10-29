//! Direct test of hover functionality

use universal_lsp::tree_sitter::TreeSitterParser;

fn main() {
    let code = r#"def greet(name: str) -> str:
    """Return a greeting message."""
    return f"Hello, {name}!"#;

    println!("Creating parser...");
    let mut parser = TreeSitterParser::new().expect("Failed to create parser");

    println!("Setting language to python...");
    parser.set_language("python").expect("Failed to set language");

    println!("Parsing code...");
    let tree = parser.parse(code, "test.py").expect("Failed to parse");

    println!("Finding node at position (0, 4) - should be 'greet'");
    // Position (0, 4) is on line 0, character 4, which is in the word "greet"
    let byte_offset = 4; // "def " = 4 bytes
    
    let root = tree.root_node();
    println!("Root node: kind={}, byte_range={:?}", root.kind(), root.byte_range());

    if let Some(node) = root.descendant_for_byte_range(byte_offset, byte_offset) {
        let node_text = &code[node.byte_range()];
        let kind = node.kind();
        println!("Found node:");
        println!("  Text: '{}'", node_text);
        println!("  Kind: {}", kind);
        println!("  Range: {:?}", node.byte_range());
    } else {
        println!("No node found at byte offset {}", byte_offset);
    }
}
