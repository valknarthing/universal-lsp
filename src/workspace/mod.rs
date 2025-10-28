//! Multi-root Workspace Support
//!
//! Manages multiple workspace folders and their configurations

use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_lsp::lsp_types::*;

/// Workspace folder information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFolder {
    pub uri: Url,
    pub name: String,
    pub config: WorkspaceConfig,
}

/// Per-workspace configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub indent_size: Option<usize>,
    pub use_tabs: Option<bool>,
    pub excluded_paths: Vec<String>,
    pub language_overrides: std::collections::HashMap<String, LanguageConfig>,
}

/// Language-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    pub indent_size: Option<usize>,
    pub formatter: Option<String>,
    pub linter: Option<String>,
}

/// Manages multiple workspace folders
#[derive(Debug)]
pub struct WorkspaceManager {
    folders: Arc<DashMap<String, WorkspaceFolder>>,
    document_to_workspace: Arc<DashMap<String, String>>,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        Self {
            folders: Arc::new(DashMap::new()),
            document_to_workspace: Arc::new(DashMap::new()),
        }
    }

    /// Add a workspace folder
    pub fn add_folder(&self, folder: tower_lsp::lsp_types::WorkspaceFolder) -> Result<()> {
        let uri_str = folder.uri.to_string();
        let config = self.load_workspace_config(&folder.uri)?;

        let workspace = WorkspaceFolder {
            uri: folder.uri,
            name: folder.name,
            config,
        };

        self.folders.insert(uri_str, workspace);
        Ok(())
    }

    /// Remove a workspace folder
    pub fn remove_folder(&self, uri: &Url) -> Result<()> {
        let uri_str = uri.to_string();
        self.folders.remove(&uri_str);

        // Remove all document mappings for this workspace
        self.document_to_workspace
            .retain(|_doc, workspace| workspace != &uri_str);

        Ok(())
    }

    /// Get workspace folder for a document
    pub fn get_workspace_for_document(&self, document_uri: &Url) -> Option<WorkspaceFolder> {
        // Check cached mapping first
        if let Some(workspace_uri) = self.document_to_workspace.get(document_uri.as_str()) {
            return self.folders.get(workspace_uri.as_str()).map(|f| f.clone());
        }

        // Find workspace by checking if document is within workspace
        let doc_path = document_uri.path();
        for folder in self.folders.iter() {
            let workspace_path = folder.uri.path();
            if doc_path.starts_with(workspace_path) {
                // Cache the mapping
                self.document_to_workspace
                    .insert(document_uri.to_string(), folder.uri.to_string());
                return Some(folder.clone());
            }
        }

        None
    }

    /// Get configuration for a document
    pub fn get_config_for_document(&self, document_uri: &Url, lang: &str) -> WorkspaceConfig {
        if let Some(workspace) = self.get_workspace_for_document(document_uri) {
            let mut config = workspace.config.clone();

            // Apply language-specific overrides
            if let Some(lang_config) = config.language_overrides.get(lang) {
                if let Some(indent) = lang_config.indent_size {
                    config.indent_size = Some(indent);
                }
            }

            config
        } else {
            WorkspaceConfig::default()
        }
    }

    /// Check if a path should be excluded
    pub fn is_excluded(&self, document_uri: &Url) -> bool {
        if let Some(workspace) = self.get_workspace_for_document(document_uri) {
            let doc_path = document_uri.path();
            for pattern in &workspace.config.excluded_paths {
                if self.matches_pattern(doc_path, pattern) {
                    return true;
                }
            }
        }
        false
    }

    /// List all workspace folders
    pub fn list_folders(&self) -> Vec<WorkspaceFolder> {
        self.folders.iter().map(|f| f.clone()).collect()
    }

    /// Get total workspace count
    pub fn count(&self) -> usize {
        self.folders.len()
    }

    /// Load workspace configuration from .universal-lsp.json or .universal-lsp.toml
    fn load_workspace_config(&self, workspace_uri: &Url) -> Result<WorkspaceConfig> {
        let workspace_path = workspace_uri.to_file_path().map_err(|_| {
            anyhow::anyhow!("Invalid workspace URI: {}", workspace_uri)
        })?;

        // Try JSON config first
        let json_config = workspace_path.join(".universal-lsp.json");
        if json_config.exists() {
            let content = std::fs::read_to_string(json_config)?;
            let config: WorkspaceConfig = serde_json::from_str(&content)?;
            return Ok(config);
        }

        // Try TOML config
        let toml_config = workspace_path.join(".universal-lsp.toml");
        if toml_config.exists() {
            let content = std::fs::read_to_string(toml_config)?;
            let config: WorkspaceConfig = toml::from_str(&content)?;
            return Ok(config);
        }

        // No config found, use defaults
        Ok(WorkspaceConfig::default())
    }

    /// Simple pattern matching (supports * wildcard)
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            // Simple glob matching
            let parts: Vec<&str> = pattern.split('*').collect();
            let mut current_pos = 0;

            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }

                if i == 0 {
                    // First part must match start
                    if !path.starts_with(part) {
                        return false;
                    }
                    current_pos = part.len();
                } else if i == parts.len() - 1 {
                    // Last part must match end
                    if !path.ends_with(part) {
                        return false;
                    }
                } else {
                    // Middle parts must exist in order
                    if let Some(pos) = path[current_pos..].find(part) {
                        current_pos += pos + part.len();
                    } else {
                        return false;
                    }
                }
            }
            true
        } else {
            path.contains(pattern)
        }
    }
}

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let manager = WorkspaceManager::new();

        assert!(manager.matches_pattern("/path/to/node_modules/file.js", "*node_modules*"));
        assert!(manager.matches_pattern("/path/to/file.test.js", "*.test.js"));
        assert!(manager.matches_pattern("/path/to/.git/config", "*/.git/*"));
        assert!(!manager.matches_pattern("/path/to/file.js", "*.test.js"));
    }

    #[test]
    fn test_workspace_management() {
        let manager = WorkspaceManager::new();

        let folder = tower_lsp::lsp_types::WorkspaceFolder {
            uri: Url::parse("file:///workspace1").unwrap(),
            name: "Workspace 1".to_string(),
        };

        assert!(manager.add_folder(folder).is_ok());
        assert_eq!(manager.count(), 1);

        let folders = manager.list_folders();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].name, "Workspace 1");
    }

    #[test]
    fn test_document_workspace_mapping() {
        let manager = WorkspaceManager::new();

        let folder = tower_lsp::lsp_types::WorkspaceFolder {
            uri: Url::parse("file:///workspace1").unwrap(),
            name: "Workspace 1".to_string(),
        };

        manager.add_folder(folder).unwrap();

        let doc_uri = Url::parse("file:///workspace1/src/file.rs").unwrap();
        let workspace = manager.get_workspace_for_document(&doc_uri);

        assert!(workspace.is_some());
        assert_eq!(workspace.unwrap().name, "Workspace 1");
    }
}
