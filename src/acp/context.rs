//! Context Provider for ACP Agent
//!
//! This module gathers rich context about the workspace to provide Claude with
//! comprehensive understanding of the development environment:
//! - Workspace structure and language
//! - Git repository status
//! - Build system configuration
//! - File tree structure
//! - Project diagnostics

use anyhow::{Context as AnyhowContext, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;

#[cfg(feature = "mcp")]
use crate::coordinator::CoordinatorClient;

/// Context provider for gathering workspace information
pub struct ContextProvider {
    workspace_root: PathBuf,
    #[cfg(feature = "mcp")]
    coordinator: Option<CoordinatorClient>,
}

impl ContextProvider {
    /// Create a new context provider
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            #[cfg(feature = "mcp")]
            coordinator: None,
        }
    }

    /// Create context provider with MCP coordinator
    #[cfg(feature = "mcp")]
    pub fn with_coordinator(workspace_root: PathBuf, coordinator: CoordinatorClient) -> Self {
        Self {
            workspace_root,
            coordinator: Some(coordinator),
        }
    }

    /// Gather all available context about the workspace
    pub async fn gather_context(&self) -> Result<Value> {
        let mut context = json!({});

        // 1. Workspace basic info
        context["workspace"] = self.get_workspace_info().await?;

        // 2. Language and build system
        if let Ok(lang) = self.detect_primary_language().await {
            context["language"] = json!(lang);
        }

        if let Ok(build_sys) = self.detect_build_system().await {
            context["build_system"] = build_sys;
        }

        // 3. Git information (if available)
        if let Ok(git_info) = self.get_git_info().await {
            context["git"] = git_info;
        }

        // 4. File structure (top-level overview)
        if let Ok(file_tree) = self.get_file_tree_summary().await {
            context["files"] = file_tree;
        }

        Ok(context)
    }

    /// Get basic workspace information
    async fn get_workspace_info(&self) -> Result<Value> {
        let workspace_name = self.workspace_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        Ok(json!({
            "root": self.workspace_root.display().to_string(),
            "name": workspace_name,
        }))
    }

    /// Detect the primary programming language of the workspace
    async fn detect_primary_language(&self) -> Result<String> {
        // Check for common language indicators
        if self.file_exists("Cargo.toml").await {
            return Ok("Rust".to_string());
        }

        if self.file_exists("package.json").await {
            // Distinguish between JS and TS
            if self.file_exists("tsconfig.json").await {
                return Ok("TypeScript".to_string());
            }
            return Ok("JavaScript".to_string());
        }

        if self.file_exists("requirements.txt").await || self.file_exists("setup.py").await || self.file_exists("pyproject.toml").await {
            return Ok("Python".to_string());
        }

        if self.file_exists("go.mod").await {
            return Ok("Go".to_string());
        }

        if self.file_exists("pom.xml").await || self.file_exists("build.gradle").await {
            return Ok("Java".to_string());
        }

        if self.file_exists("Gemfile").await {
            return Ok("Ruby".to_string());
        }

        if self.file_exists("composer.json").await {
            return Ok("PHP".to_string());
        }

        // Count files by extension as fallback
        self.detect_language_by_extension().await
    }

    /// Detect language by counting file extensions
    async fn detect_language_by_extension(&self) -> Result<String> {
        let mut extension_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        if let Ok(mut entries) = fs::read_dir(&self.workspace_root).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(file_type) = entry.file_type().await {
                    if file_type.is_file() {
                        if let Some(ext) = entry.path().extension() {
                            let ext_str = ext.to_string_lossy().to_lowercase();
                            *extension_counts.entry(ext_str).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        // Map extensions to languages
        let ext_to_lang = [
            ("rs", "Rust"),
            ("js", "JavaScript"),
            ("ts", "TypeScript"),
            ("tsx", "TypeScript"),
            ("jsx", "JavaScript"),
            ("py", "Python"),
            ("go", "Go"),
            ("java", "Java"),
            ("rb", "Ruby"),
            ("php", "PHP"),
            ("c", "C"),
            ("cpp", "C++"),
            ("h", "C/C++"),
            ("cs", "C#"),
        ];

        let mut lang_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for (ext, count) in extension_counts.iter() {
            if let Some((_, lang)) = ext_to_lang.iter().find(|(e, _)| e == &ext.as_str()) {
                *lang_counts.entry(lang).or_insert(0) += count;
            }
        }

        lang_counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang.to_string())
            .ok_or_else(|| anyhow::anyhow!("Could not detect primary language"))
    }

    /// Detect the build system used in the workspace
    async fn detect_build_system(&self) -> Result<Value> {
        let mut build_systems = vec![];

        if self.file_exists("Cargo.toml").await {
            build_systems.push(json!({
                "name": "Cargo",
                "type": "rust",
                "config": "Cargo.toml"
            }));
        }

        if self.file_exists("package.json").await {
            let package_managers = if self.file_exists("pnpm-lock.yaml").await {
                "pnpm"
            } else if self.file_exists("yarn.lock").await {
                "yarn"
            } else if self.file_exists("package-lock.json").await {
                "npm"
            } else {
                "npm (no lockfile)"
            };

            build_systems.push(json!({
                "name": "Node.js",
                "type": "javascript",
                "package_manager": package_managers,
                "config": "package.json"
            }));
        }

        if self.file_exists("Makefile").await {
            build_systems.push(json!({
                "name": "Make",
                "type": "generic",
                "config": "Makefile"
            }));
        }

        if self.file_exists("CMakeLists.txt").await {
            build_systems.push(json!({
                "name": "CMake",
                "type": "c/c++",
                "config": "CMakeLists.txt"
            }));
        }

        if self.file_exists("pom.xml").await {
            build_systems.push(json!({
                "name": "Maven",
                "type": "java",
                "config": "pom.xml"
            }));
        }

        if self.file_exists("build.gradle").await || self.file_exists("build.gradle.kts").await {
            build_systems.push(json!({
                "name": "Gradle",
                "type": "java",
                "config": "build.gradle"
            }));
        }

        if build_systems.is_empty() {
            Ok(json!(null))
        } else if build_systems.len() == 1 {
            Ok(build_systems.into_iter().next().unwrap())
        } else {
            Ok(json!(build_systems))
        }
    }

    /// Get git repository information
    async fn get_git_info(&self) -> Result<Value> {
        let git_dir = self.workspace_root.join(".git");

        if !git_dir.exists() {
            return Ok(json!({
                "status": "not a git repository"
            }));
        }

        let mut git_info = json!({
            "status": "git repository"
        });

        // Try to get current branch
        if let Ok(branch) = self.get_current_branch().await {
            git_info["branch"] = json!(branch);
        }

        // Check for common git files
        if self.file_exists(".gitignore").await {
            git_info["has_gitignore"] = json!(true);
        }

        git_info["directory"] = json!(".git");

        Ok(git_info)
    }

    /// Get current git branch name
    async fn get_current_branch(&self) -> Result<String> {
        let head_file = self.workspace_root.join(".git/HEAD");

        if let Ok(content) = fs::read_to_string(&head_file).await {
            // HEAD file format: "ref: refs/heads/main" or a commit hash
            if let Some(branch_ref) = content.strip_prefix("ref: refs/heads/") {
                return Ok(branch_ref.trim().to_string());
            } else if content.len() == 40 || content.len() == 41 {
                // Detached HEAD state (commit hash)
                return Ok(format!("detached at {}", &content[..7]));
            }
        }

        Ok("unknown".to_string())
    }

    /// Get a summary of the file tree structure
    async fn get_file_tree_summary(&self) -> Result<Value> {
        let mut directories = vec![];
        let mut files = vec![];
        let mut total_files = 0;
        let mut total_dirs = 0;

        if let Ok(mut entries) = fs::read_dir(&self.workspace_root).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let file_name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files and common ignores
                if file_name.starts_with('.')
                    || file_name == "node_modules"
                    || file_name == "target"
                    || file_name == "__pycache__"
                    || file_name == "dist"
                    || file_name == "build" {
                    continue;
                }

                if let Ok(file_type) = entry.file_type().await {
                    if file_type.is_dir() {
                        directories.push(file_name);
                        total_dirs += 1;
                    } else if file_type.is_file() {
                        // Only include important files in summary
                        if self.is_important_file(&file_name) {
                            files.push(file_name);
                        }
                        total_files += 1;
                    }
                }
            }
        }

        directories.sort();
        files.sort();

        Ok(json!({
            "top_level_dirs": directories,
            "important_files": files,
            "total_files": total_files,
            "total_dirs": total_dirs,
        }))
    }

    /// Check if a file is considered "important" for context
    fn is_important_file(&self, filename: &str) -> bool {
        let important_files = [
            "README.md", "README", "README.txt",
            "LICENSE", "LICENSE.md", "LICENSE.txt",
            "Cargo.toml", "Cargo.lock",
            "package.json", "package-lock.json", "pnpm-lock.yaml", "yarn.lock",
            "go.mod", "go.sum",
            "requirements.txt", "setup.py", "pyproject.toml",
            "Makefile", "CMakeLists.txt",
            "Dockerfile", "docker-compose.yml",
            ".gitignore", ".dockerignore",
            "tsconfig.json", "jsconfig.json",
        ];

        important_files.contains(&filename)
    }

    /// Check if a file exists in the workspace
    async fn file_exists(&self, filename: &str) -> bool {
        self.workspace_root.join(filename).exists()
    }

    /// Format context as a human-readable string for Claude's system prompt
    pub async fn format_for_prompt(&self) -> String {
        match self.gather_context().await {
            Ok(context) => {
                let mut prompt = String::from("\n## Workspace Context\n");

                // Workspace info
                if let Some(workspace) = context.get("workspace") {
                    if let Some(name) = workspace.get("name").and_then(|v| v.as_str()) {
                        prompt.push_str(&format!("**Project**: {}\n", name));
                    }
                    if let Some(root) = workspace.get("root").and_then(|v| v.as_str()) {
                        prompt.push_str(&format!("**Location**: {}\n", root));
                    }
                }

                // Language
                if let Some(lang) = context.get("language").and_then(|v| v.as_str()) {
                    prompt.push_str(&format!("**Primary Language**: {}\n", lang));
                }

                // Build system
                if let Some(build) = context.get("build_system") {
                    if !build.is_null() {
                        if let Some(name) = build.get("name").and_then(|v| v.as_str()) {
                            prompt.push_str(&format!("**Build System**: {}\n", name));
                        }
                    }
                }

                // Git info
                if let Some(git) = context.get("git") {
                    if let Some(branch) = git.get("branch").and_then(|v| v.as_str()) {
                        prompt.push_str(&format!("**Git Branch**: {}\n", branch));
                    }
                }

                // File structure
                if let Some(files) = context.get("files") {
                    if let Some(total) = files.get("total_files").and_then(|v| v.as_u64()) {
                        prompt.push_str(&format!("**Total Files**: {}\n", total));
                    }
                    if let Some(dirs) = files.get("top_level_dirs").and_then(|v| v.as_array()) {
                        if !dirs.is_empty() {
                            prompt.push_str(&format!("**Top Directories**: {}\n",
                                dirs.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                    }
                }

                prompt.push('\n');
                prompt
            }
            Err(e) => {
                format!("\n## Workspace Context\n*Error gathering context: {}*\n\n", e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_context_provider_creation() {
        let temp_dir = TempDir::new().unwrap();
        let provider = ContextProvider::new(temp_dir.path().to_path_buf());

        let context = provider.gather_context().await.unwrap();
        assert!(context.is_object());
        assert!(context.get("workspace").is_some());
    }

    #[tokio::test]
    async fn test_detect_rust_project() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let provider = ContextProvider::new(temp_dir.path().to_path_buf());
        let lang = provider.detect_primary_language().await.unwrap();
        assert_eq!(lang, "Rust");
    }

    #[tokio::test]
    async fn test_detect_javascript_project() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("package.json"), r#"{"name": "test"}"#)
            .await
            .unwrap();

        let provider = ContextProvider::new(temp_dir.path().to_path_buf());
        let lang = provider.detect_primary_language().await.unwrap();
        assert_eq!(lang, "JavaScript");
    }

    #[tokio::test]
    async fn test_detect_typescript_project() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("package.json"), r#"{"name": "test"}"#)
            .await
            .unwrap();
        fs::write(temp_dir.path().join("tsconfig.json"), "{}")
            .await
            .unwrap();

        let provider = ContextProvider::new(temp_dir.path().to_path_buf());
        let lang = provider.detect_primary_language().await.unwrap();
        assert_eq!(lang, "TypeScript");
    }

    #[tokio::test]
    async fn test_detect_build_system_cargo() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let provider = ContextProvider::new(temp_dir.path().to_path_buf());
        let build_sys = provider.detect_build_system().await.unwrap();

        assert_eq!(build_sys["name"], "Cargo");
        assert_eq!(build_sys["type"], "rust");
    }

    #[tokio::test]
    async fn test_git_info_no_repo() {
        let temp_dir = TempDir::new().unwrap();
        let provider = ContextProvider::new(temp_dir.path().to_path_buf());

        let git_info = provider.get_git_info().await.unwrap();
        assert_eq!(git_info["status"], "not a git repository");
    }

    #[tokio::test]
    async fn test_file_tree_summary() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("README.md"), "# Test")
            .await
            .unwrap();
        fs::write(temp_dir.path().join("test.txt"), "content")
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("src"))
            .await
            .unwrap();

        let provider = ContextProvider::new(temp_dir.path().to_path_buf());
        let tree = provider.get_file_tree_summary().await.unwrap();

        assert!(tree["important_files"].as_array().unwrap().len() > 0);
        assert!(tree["top_level_dirs"].as_array().unwrap().contains(&json!("src")));
    }

    #[tokio::test]
    async fn test_format_for_prompt() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"")
            .await
            .unwrap();

        let provider = ContextProvider::new(temp_dir.path().to_path_buf());
        let prompt = provider.format_for_prompt().await;

        assert!(prompt.contains("Workspace Context"));
        assert!(prompt.contains("Rust"));
    }
}
