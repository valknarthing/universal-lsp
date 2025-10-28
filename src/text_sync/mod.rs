//! Incremental Text Synchronization
//!
//! Efficiently tracks document changes using incremental updates

use anyhow::Result;
use tower_lsp::lsp_types::*;

/// Represents a document with incremental change tracking
#[derive(Debug, Clone)]
pub struct IncrementalDocument {
    pub uri: String,
    pub content: String,
    pub version: i32,
    pub line_offsets: Vec<usize>,
}

impl IncrementalDocument {
    /// Create a new document
    pub fn new(uri: String, content: String, version: i32) -> Self {
        let line_offsets = Self::compute_line_offsets(&content);
        Self {
            uri,
            content,
            version,
            line_offsets,
        }
    }

    /// Apply incremental changes to the document
    pub fn apply_changes(&mut self, changes: Vec<TextDocumentContentChangeEvent>, version: i32) -> Result<()> {
        self.version = version;

        for change in changes {
            if let Some(range) = change.range {
                // Incremental change
                self.apply_incremental_change(range, &change.text)?;
            } else {
                // Full document sync
                self.content = change.text;
                self.line_offsets = Self::compute_line_offsets(&self.content);
            }
        }

        Ok(())
    }

    /// Apply an incremental change to a specific range
    fn apply_incremental_change(&mut self, range: Range, new_text: &str) -> Result<()> {
        let start_offset = self.position_to_offset(range.start)?;
        let end_offset = self.position_to_offset(range.end)?;

        // Replace the text in the range
        let mut new_content = String::with_capacity(
            self.content.len() - (end_offset - start_offset) + new_text.len()
        );
        new_content.push_str(&self.content[..start_offset]);
        new_content.push_str(new_text);
        new_content.push_str(&self.content[end_offset..]);

        self.content = new_content;
        self.line_offsets = Self::compute_line_offsets(&self.content);

        Ok(())
    }

    /// Convert LSP Position to byte offset
    pub fn position_to_offset(&self, position: Position) -> Result<usize> {
        let line = position.line as usize;

        if line >= self.line_offsets.len() {
            return Err(anyhow::anyhow!("Line {} is out of bounds", line));
        }

        let line_start = self.line_offsets[line];
        let mut char_count = 0;
        let mut byte_offset = line_start;

        for ch in self.content[line_start..].chars() {
            if ch == '\n' {
                break;
            }
            if char_count == position.character as usize {
                return Ok(byte_offset);
            }
            byte_offset += ch.len_utf8();
            char_count += 1;
        }

        // If we reached end of line
        if char_count == position.character as usize {
            Ok(byte_offset)
        } else {
            Err(anyhow::anyhow!(
                "Character {} is out of bounds on line {}",
                position.character,
                line
            ))
        }
    }

    /// Convert byte offset to LSP Position
    pub fn offset_to_position(&self, offset: usize) -> Result<Position> {
        if offset > self.content.len() {
            return Err(anyhow::anyhow!("Offset {} is out of bounds", offset));
        }

        // Binary search for the line
        let line = match self.line_offsets.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };

        let line_start = self.line_offsets[line];
        let mut character = 0;

        for ch in self.content[line_start..offset].chars() {
            if ch == '\n' {
                break;
            }
            character += 1;
        }

        Ok(Position {
            line: line as u32,
            character: character as u32,
        })
    }

    /// Get text in a specific range
    pub fn get_text_in_range(&self, range: Range) -> Result<String> {
        let start_offset = self.position_to_offset(range.start)?;
        let end_offset = self.position_to_offset(range.end)?;
        Ok(self.content[start_offset..end_offset].to_string())
    }

    /// Get the entire document content
    pub fn get_content(&self) -> &str {
        &self.content
    }

    /// Get line count
    pub fn line_count(&self) -> usize {
        self.line_offsets.len()
    }

    /// Compute byte offsets for each line
    fn compute_line_offsets(content: &str) -> Vec<usize> {
        let mut offsets = vec![0];
        let mut current_offset = 0;

        for ch in content.chars() {
            current_offset += ch.len_utf8();
            if ch == '\n' {
                offsets.push(current_offset);
            }
        }

        offsets
    }
}

/// Manager for incremental document synchronization
#[derive(Debug)]
pub struct TextSyncManager {
    documents: dashmap::DashMap<String, IncrementalDocument>,
}

impl TextSyncManager {
    pub fn new() -> Self {
        Self {
            documents: dashmap::DashMap::new(),
        }
    }

    /// Handle document open
    pub fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = IncrementalDocument::new(
            params.text_document.uri.to_string(),
            params.text_document.text,
            params.text_document.version,
        );
        self.documents.insert(params.text_document.uri.to_string(), doc);
    }

    /// Handle document change
    pub fn did_change(&self, params: DidChangeTextDocumentParams) -> Result<()> {
        let uri = params.text_document.uri.to_string();

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            doc.apply_changes(params.content_changes, params.text_document.version)?;
        } else {
            return Err(anyhow::anyhow!("Document not found: {}", uri));
        }

        Ok(())
    }

    /// Handle document close
    pub fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri.to_string());
    }

    /// Get document content
    pub fn get_document(&self, uri: &str) -> Option<IncrementalDocument> {
        self.documents.get(uri).map(|doc| doc.clone())
    }

    /// Get document content as string
    pub fn get_content(&self, uri: &str) -> Option<String> {
        self.documents.get(uri).map(|doc| doc.content.clone())
    }

    /// Get all document URIs
    pub fn list_documents(&self) -> Vec<String> {
        self.documents.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for TextSyncManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_offsets() {
        let doc = IncrementalDocument::new(
            "test.txt".to_string(),
            "line1\nline2\nline3".to_string(),
            1,
        );

        assert_eq!(doc.line_offsets, vec![0, 6, 12]);
        assert_eq!(doc.line_count(), 3);
    }

    #[test]
    fn test_position_to_offset() {
        let doc = IncrementalDocument::new(
            "test.txt".to_string(),
            "line1\nline2\nline3".to_string(),
            1,
        );

        let pos = Position { line: 0, character: 0 };
        assert_eq!(doc.position_to_offset(pos).unwrap(), 0);

        let pos = Position { line: 1, character: 0 };
        assert_eq!(doc.position_to_offset(pos).unwrap(), 6);

        let pos = Position { line: 1, character: 3 };
        assert_eq!(doc.position_to_offset(pos).unwrap(), 9);
    }

    #[test]
    fn test_offset_to_position() {
        let doc = IncrementalDocument::new(
            "test.txt".to_string(),
            "line1\nline2\nline3".to_string(),
            1,
        );

        assert_eq!(
            doc.offset_to_position(0).unwrap(),
            Position { line: 0, character: 0 }
        );

        assert_eq!(
            doc.offset_to_position(6).unwrap(),
            Position { line: 1, character: 0 }
        );

        assert_eq!(
            doc.offset_to_position(9).unwrap(),
            Position { line: 1, character: 3 }
        );
    }

    #[test]
    fn test_incremental_change() {
        let mut doc = IncrementalDocument::new(
            "test.txt".to_string(),
            "line1\nline2\nline3".to_string(),
            1,
        );

        let change = TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 1, character: 0 },
                end: Position { line: 1, character: 5 },
            }),
            range_length: None,
            text: "HELLO".to_string(),
        };

        doc.apply_changes(vec![change], 2).unwrap();
        assert_eq!(doc.content, "line1\nHELLO\nline3");
    }

    #[test]
    fn test_get_text_in_range() {
        let doc = IncrementalDocument::new(
            "test.txt".to_string(),
            "line1\nline2\nline3".to_string(),
            1,
        );

        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 5 },
        };

        assert_eq!(doc.get_text_in_range(range).unwrap(), "line1");
    }

    #[test]
    fn test_full_document_sync() {
        let mut doc = IncrementalDocument::new(
            "test.txt".to_string(),
            "old content".to_string(),
            1,
        );

        let change = TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "new content".to_string(),
        };

        doc.apply_changes(vec![change], 2).unwrap();
        assert_eq!(doc.content, "new content");
        assert_eq!(doc.version, 2);
    }
}
