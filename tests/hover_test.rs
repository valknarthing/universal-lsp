//! Test hover functionality with tree-sitter

use universal_lsp::tree_sitter::TreeSitterParser;

#[test]
fn test_python_hover() {
    let code = r#"def greet(name: str) -> str:
    """Return a greeting message."""
    return f"Hello, {name}!"

message = greet("World")
print(message)"#;

    eprintln!("Creating parser...");
    let mut parser = TreeSitterParser::new().expect("Failed to create parser");

    eprintln!("Setting language to python...");
    parser.set_language("python").expect("Failed to set language");

    eprintln!("Parsing code...");
    let tree = parser.parse(code, "test.py").expect("Failed to parse");

    eprintln!("Root node: kind={}, byte_range={:?}", tree.root_node().kind(), tree.root_node().byte_range());

    // Test hovering over "greet" function name (line 0, character 4-9)
    let byte_offset = 4; // "def " = 4 bytes, points to 'g' in "greet"

    eprintln!("Finding node at byte offset {}...", byte_offset);
    let node = tree.root_node().descendant_for_byte_range(byte_offset, byte_offset)
        .expect("Should find node at offset 4");

    let node_text = &code[node.byte_range()];
    let kind = node.kind();

    eprintln!("Found node:");
    eprintln!("  Text: '{}'", node_text);
    eprintln!("  Kind: {}", kind);
    eprintln!("  Byte range: {:?}", node.byte_range());

    // Verify we found something reasonable
    assert!(!node_text.is_empty(), "Node text should not be empty");
    assert!(!kind.is_empty(), "Node kind should not be empty");

    // Now test at line 4, character 10 (the call to "greet")
    // Line 4 is: "message = greet("World")"
    // We need to calculate byte offset: 3 lines before + "message = " = ?
    let line_4_start = code.lines().take(4).map(|l| l.len() + 1).sum::<usize>();
    let byte_offset_call = line_4_start + 10; // "message = " is 10 chars

    eprintln!("\nFinding node at line 4, character 10 (greet call), byte offset {}...", byte_offset_call);
    if let Some(node) = tree.root_node().descendant_for_byte_range(byte_offset_call, byte_offset_call) {
        let node_text = &code[node.byte_range()];
        let kind = node.kind();

        eprintln!("Found node:");
        eprintln!("  Text: '{}'", node_text);
        eprintln!("  Kind: {}", kind);
        eprintln!("  Byte range: {:?}", node.byte_range());

        assert!(!node_text.is_empty(), "Node text should not be empty");
    } else {
        panic!("Should find node at byte offset {}", byte_offset_call);
    }
}

#[test]
fn test_position_to_byte() {
    let code = "def greet(name):\n    return name\n";

    // Line 0, char 0 = byte 0
    let offset = position_to_byte(code, 0, 0);
    assert_eq!(offset, 0);

    // Line 0, char 4 = byte 4 (pointing to 'g' in greet)
    let offset = position_to_byte(code, 0, 4);
    assert_eq!(offset, 4);

    // Line 1, char 0 = byte 17 (after "def greet(name):\n")
    let offset = position_to_byte(code, 1, 0);
    assert_eq!(offset, 17);

    // Line 1, char 4 = byte 21 (after spaces)
    let offset = position_to_byte(code, 1, 4);
    assert_eq!(offset, 21);
}

fn position_to_byte(source: &str, line: u32, character: u32) -> usize {
    let mut byte_offset = 0;
    let mut current_line = 0;
    let mut current_char = 0;

    for ch in source.chars() {
        if current_line == line && current_char == character {
            break;
        }
        byte_offset += ch.len_utf8();
        current_char += 1;
        if ch == '\n' {
            current_line += 1;
            current_char = 0;
        }
    }

    byte_offset
}
