//! Workspace Symbol Indexing
//!
//! This module provides comprehensive workspace-wide symbol indexing for enhanced
//! AI context and semantic search capabilities.

use anyhow::{Context, Result};
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;
use tower_lsp::lsp_types::{Location, SymbolKind, Url};
use tracing::{debug, info, warn};

use crate::language::detect_language;
use crate::tree_sitter::TreeSitterParser;

/// Maximum number of files to index
const MAX_INDEXED_FILES: usize = 10_000;

/// File patterns to exclude from indexing
const EXCLUDE_PATTERNS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".next",
    ".cache",
    "coverage",
    "vendor",
    "__pycache__",
];

/// Indexed symbol with metadata
#[derive(Debug, Clone)]
pub struct IndexedSymbol {
    /// Symbol name
    pub name: String,
    /// Symbol kind (function, class, variable, etc.)
    pub kind: SymbolKind,
    /// File location
    pub location: Location,
    /// Container name (parent class/namespace)
    pub container: Option<String>,
    /// Symbol signature (for functions)
    pub signature: Option<String>,
    /// Documentation snippet
    pub documentation: Option<String>,
    /// Last indexed timestamp
    pub indexed_at: SystemTime,
}

impl IndexedSymbol {
    /// Get a searchable text representation
    pub fn searchable_text(&self) -> String {
        let mut text = format!("{} {}", self.name, symbol_kind_name(self.kind));

        if let Some(container) = &self.container {
            text.push_str(&format!(" in {}", container));
        }

        if let Some(sig) = &self.signature {
            text.push_str(&format!(" {}", sig));
        }

        if let Some(doc) = &self.documentation {
            text.push_str(&format!(" {}", doc));
        }

        text
    }
}

/// Workspace symbol index
#[derive(Clone)]
pub struct WorkspaceIndex {
    /// Indexed symbols by file URI
    symbols_by_file: Arc<DashMap<String, Vec<IndexedSymbol>>>,
    /// Global symbol lookup by name (for fast search)
    symbols_by_name: Arc<DashMap<String, Vec<IndexedSymbol>>>,
    /// Workspace root path
    workspace_root: Arc<Option<PathBuf>>,
    /// Last full index timestamp
    last_indexed: Arc<std::sync::RwLock<Option<SystemTime>>>,
    /// Parser cache
    parsers: Arc<DashMap<String, TreeSitterParser>>,
}

impl WorkspaceIndex {
    /// Create a new workspace index
    pub fn new() -> Self {
        Self {
            symbols_by_file: Arc::new(DashMap::new()),
            symbols_by_name: Arc::new(DashMap::new()),
            workspace_root: Arc::new(None),
            last_indexed: Arc::new(std::sync::RwLock::new(None)),
            parsers: Arc::new(DashMap::new()),
        }
    }

    /// Set workspace root
    pub fn set_workspace_root(&mut self, root: PathBuf) {
        self.workspace_root = Arc::new(Some(root));
    }

    /// Index the entire workspace
    pub async fn index_workspace(&self) -> Result<usize> {
        let root = match &*self.workspace_root {
            Some(root) => root.clone(),
            None => {
                warn!("No workspace root set, skipping index");
                return Ok(0);
            }
        };

        info!("Starting workspace indexing: {}", root.display());
        let start_time = std::time::Instant::now();

        // Clear existing index
        self.symbols_by_file.clear();
        self.symbols_by_name.clear();

        // Scan workspace for files
        let files = self.scan_workspace(&root).await?;
        info!("Found {} files to index", files.len());

        // Index each file
        let mut total_symbols = 0;
        for file_path in files.iter().take(MAX_INDEXED_FILES) {
            match self.index_file(file_path).await {
                Ok(count) => total_symbols += count,
                Err(e) => {
                    debug!("Failed to index {}: {}", file_path.display(), e);
                }
            }
        }

        // Update last indexed timestamp
        *self.last_indexed.write().unwrap() = Some(SystemTime::now());

        let elapsed = start_time.elapsed();
        info!(
            "Workspace indexing complete: {} symbols in {} files ({:.2}s)",
            total_symbols,
            files.len().min(MAX_INDEXED_FILES),
            elapsed.as_secs_f64()
        );

        Ok(total_symbols)
    }

    /// Scan workspace for code files
    async fn scan_workspace(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.scan_directory(root, &mut files).await?;
        Ok(files)
    }

    /// Recursively scan directory
    fn scan_directory<'a>(
        &'a self,
        dir: &'a Path,
        files: &'a mut Vec<PathBuf>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // Check if directory should be excluded
            if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
                if EXCLUDE_PATTERNS.contains(&dir_name) {
                    return Ok(());
                }
            }

            let mut entries = match fs::read_dir(dir).await {
                Ok(entries) => entries,
                Err(e) => {
                    debug!("Failed to read directory {}: {}", dir.display(), e);
                    return Ok(());
                }
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();

                if path.is_dir() {
                    self.scan_directory(&path, files).await?;
                } else if path.is_file() {
                    // Check if file is a source code file
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let lang = detect_language(&format!("file.{}", ext));
                        if lang != "plaintext" && lang != "unknown" {
                            files.push(path);
                        }
                    }
                }
            }

            Ok(())
        })
    }

    /// Index a single file
    pub async fn index_file(&self, file_path: &Path) -> Result<usize> {
        // Read file content
        let content = fs::read_to_string(file_path)
            .await
            .context("Failed to read file")?;

        // Detect language
        let lang = detect_language(file_path.to_str().unwrap_or(""));

        // Get or create parser
        if !self.parsers.contains_key(&lang.to_string()) {
            match TreeSitterParser::new() {
                Ok(mut p) => {
                    let _ = p.set_language(&lang);
                    self.parsers.insert(lang.to_string(), p);
                }
                Err(_) => {
                    // Fallback: create a parser without setting language
                    if let Ok(p) = TreeSitterParser::new() {
                        self.parsers.insert(lang.to_string(), p);
                    }
                }
            }
        }

        // Get a mutable reference to the parser
        let mut parser = match self.parsers.get_mut(&lang.to_string()) {
            Some(p) => p,
            None => return Ok(0), // Skip if parser creation failed
        };

        // Parse file
        let uri_str = format!("file://{}", file_path.display());
        let tree = parser.parse(&content, &uri_str)?;

        // Extract symbols
        let symbols = parser.extract_symbols(&tree, &content, &lang)?;

        // Convert to indexed symbols
        let mut indexed_symbols = Vec::new();
        let uri = Url::parse(&uri_str).context("Failed to parse URI")?;

        for symbol in symbols {
            let indexed = IndexedSymbol {
                name: symbol.name.clone(),
                kind: symbol.kind,
                location: Location {
                    uri: uri.clone(),
                    range: symbol.range,
                },
                container: symbol.detail.clone(), // Use detail as container info
                signature: None, // TODO: Extract from docstring
                documentation: None,
                indexed_at: SystemTime::now(),
            };
            indexed_symbols.push(indexed);
        }

        let symbol_count = indexed_symbols.len();

        // Store in index
        self.symbols_by_file
            .insert(uri_str.clone(), indexed_symbols.clone());

        // Index by name for fast lookup
        for symbol in indexed_symbols {
            self.symbols_by_name
                .entry(symbol.name.clone())
                .or_insert_with(Vec::new)
                .push(symbol);
        }

        Ok(symbol_count)
    }

    /// Search symbols by name or pattern
    pub fn search_symbols(&self, query: &str) -> Vec<IndexedSymbol> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        // Search by exact name first
        if let Some(symbols) = self.symbols_by_name.get(query) {
            results.extend(symbols.clone());
        }

        // Search by prefix/contains
        for entry in self.symbols_by_name.iter() {
            let name_lower = entry.key().to_lowercase();
            if name_lower != query_lower
                && (name_lower.starts_with(&query_lower) || name_lower.contains(&query_lower))
            {
                results.extend(entry.value().clone());
            }
        }

        // Limit results
        results.truncate(100);
        results
    }

    /// Get all symbols in a file
    pub fn get_file_symbols(&self, uri: &str) -> Option<Vec<IndexedSymbol>> {
        self.symbols_by_file.get(uri).map(|s| s.clone())
    }

    /// Get workspace context for Claude
    ///
    /// Returns a formatted string with symbol information suitable for AI context
    pub fn get_context_for_claude(&self, query: Option<&str>) -> String {
        let mut context = String::from("# Workspace Symbol Index\n\n");

        if let Some(query) = query {
            // Focused context for specific query
            let symbols = self.search_symbols(query);
            if !symbols.is_empty() {
                context.push_str(&format!("## Symbols matching '{}'\n\n", query));
                for symbol in symbols.iter().take(20) {
                    context.push_str(&format!(
                        "- `{}` ({}) at {}:{}\n",
                        symbol.name,
                        symbol_kind_name(symbol.kind),
                        symbol.location.uri,
                        symbol.location.range.start.line
                    ));
                    if let Some(container) = &symbol.container {
                        context.push_str(&format!("  in {}\n", container));
                    }
                }
            } else {
                context.push_str("No symbols found matching query.\n");
            }
        } else {
            // General workspace overview
            let total_files = self.symbols_by_file.len();
            let total_symbols: usize = self.symbols_by_file.iter().map(|e| e.value().len()).sum();

            context.push_str(&format!(
                "Total: {} symbols across {} files\n\n",
                total_symbols, total_files
            ));

            // Sample of symbol types
            let mut kind_counts = std::collections::HashMap::new();
            for entry in self.symbols_by_name.iter() {
                for symbol in entry.value() {
                    // Use symbol kind name as HashMap key
                    let kind_name = symbol_kind_name(symbol.kind);
                    *kind_counts.entry(kind_name).or_insert(0) += 1;
                }
            }

            context.push_str("## Symbol Distribution\n\n");
            for (kind_name, count) in kind_counts.iter() {
                context.push_str(&format!("- {}: {}\n", kind_name, count));
            }
        }

        context
    }

    /// Get statistics about the index
    pub fn get_statistics(&self) -> IndexStatistics {
        let total_files = self.symbols_by_file.len();
        let total_symbols: usize = self.symbols_by_file.iter().map(|e| e.value().len()).sum();
        let last_indexed = *self.last_indexed.read().unwrap();

        IndexStatistics {
            total_files,
            total_symbols,
            last_indexed,
        }
    }

    /// Clear the entire index
    pub fn clear(&self) {
        self.symbols_by_file.clear();
        self.symbols_by_name.clear();
        *self.last_indexed.write().unwrap() = None;
    }
}

impl Default for WorkspaceIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStatistics {
    pub total_files: usize,
    pub total_symbols: usize,
    pub last_indexed: Option<SystemTime>,
}

/// Get human-readable name for symbol kind
fn symbol_kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::FILE => "File",
        SymbolKind::MODULE => "Module",
        SymbolKind::NAMESPACE => "Namespace",
        SymbolKind::PACKAGE => "Package",
        SymbolKind::CLASS => "Class",
        SymbolKind::METHOD => "Method",
        SymbolKind::PROPERTY => "Property",
        SymbolKind::FIELD => "Field",
        SymbolKind::CONSTRUCTOR => "Constructor",
        SymbolKind::ENUM => "Enum",
        SymbolKind::INTERFACE => "Interface",
        SymbolKind::FUNCTION => "Function",
        SymbolKind::VARIABLE => "Variable",
        SymbolKind::CONSTANT => "Constant",
        SymbolKind::STRING => "String",
        SymbolKind::NUMBER => "Number",
        SymbolKind::BOOLEAN => "Boolean",
        SymbolKind::ARRAY => "Array",
        SymbolKind::OBJECT => "Object",
        SymbolKind::KEY => "Key",
        SymbolKind::NULL => "Null",
        SymbolKind::ENUM_MEMBER => "EnumMember",
        SymbolKind::STRUCT => "Struct",
        SymbolKind::EVENT => "Event",
        SymbolKind::OPERATOR => "Operator",
        SymbolKind::TYPE_PARAMETER => "TypeParameter",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Range;

    #[tokio::test]
    async fn test_workspace_index_creation() {
        let index = WorkspaceIndex::new();
        let stats = index.get_statistics();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_symbols, 0);
    }

    #[test]
    fn test_symbol_searchable_text() {
        let symbol = IndexedSymbol {
            name: "calculate_sum".to_string(),
            kind: SymbolKind::FUNCTION,
            location: Location {
                uri: Url::parse("file:///test.rs").unwrap(),
                range: Range::default(),
            },
            container: Some("MathUtils".to_string()),
            signature: Some("(a: i32, b: i32) -> i32".to_string()),
            documentation: Some("Calculates the sum".to_string()),
            indexed_at: SystemTime::now(),
        };

        let text = symbol.searchable_text();
        assert!(text.contains("calculate_sum"));
        assert!(text.contains("Function"));
        assert!(text.contains("MathUtils"));
    }

    #[test]
    fn test_symbol_kind_name() {
        assert_eq!(symbol_kind_name(SymbolKind::FUNCTION), "Function");
        assert_eq!(symbol_kind_name(SymbolKind::CLASS), "Class");
        assert_eq!(symbol_kind_name(SymbolKind::VARIABLE), "Variable");
    }
}
