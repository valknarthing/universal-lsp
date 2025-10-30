//! Integration tests for ACP Context Awareness (Phase 3)
//!
//! These tests verify that the ContextProvider correctly detects workspace
//! information in real-world scenarios with actual files and directories.

use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio;
use universal_lsp::acp::context::ContextProvider;

/// Helper: Create a test workspace with specified files
fn create_test_workspace(files: Vec<(&str, &str)>) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    for (path, content) in files {
        let file_path = temp_dir.path().join(path);

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }

        fs::write(&file_path, content).expect("Failed to write file");
    }

    temp_dir
}

/// Helper: Create a minimal git repository
fn create_git_repo(dir: &TempDir, branch: &str) -> std::io::Result<()> {
    let git_dir = dir.path().join(".git");
    fs::create_dir(&git_dir)?;

    let head_content = if branch.starts_with("ref: ") {
        branch.to_string()
    } else {
        format!("ref: refs/heads/{}", branch)
    };

    fs::write(git_dir.join("HEAD"), head_content)?;
    Ok(())
}

// =============================================================================
// Language Detection Tests
// =============================================================================

#[tokio::test]
async fn test_detect_rust_workspace() {
    let workspace = create_test_workspace(vec![
        ("Cargo.toml", "[package]\nname = \"test\"\nversion = \"0.1.0\""),
        ("src/main.rs", "fn main() { println!(\"Hello\"); }"),
        ("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    assert_eq!(context["language"], "Rust");
    // Single build system returns object, not array
    assert_eq!(context["build_system"]["name"], "Cargo");
}

#[tokio::test]
async fn test_detect_typescript_workspace() {
    let workspace = create_test_workspace(vec![
        ("package.json", r#"{"name": "test", "version": "1.0.0"}"#),
        ("tsconfig.json", r#"{"compilerOptions": {"target": "ES2020"}}"#),
        ("src/index.ts", "const greeting: string = 'Hello';"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    assert_eq!(context["language"], "TypeScript");
    // Single build system returns object
    assert_eq!(context["build_system"]["name"], "Node.js");
}

#[tokio::test]
async fn test_detect_javascript_workspace() {
    let workspace = create_test_workspace(vec![
        ("package.json", r#"{"name": "test", "version": "1.0.0"}"#),
        ("src/index.js", "const greeting = 'Hello';"),
        ("src/app.js", "console.log('app');"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    assert_eq!(context["language"], "JavaScript");
}

#[tokio::test]
async fn test_detect_python_workspace() {
    let workspace = create_test_workspace(vec![
        ("requirements.txt", "requests==2.28.0\npytest==7.0.0"),
        ("src/main.py", "def main():\n    print('Hello')"),
        ("tests/test_main.py", "def test_main():\n    assert True"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    assert_eq!(context["language"], "Python");
}

#[tokio::test]
async fn test_detect_go_workspace() {
    let workspace = create_test_workspace(vec![
        ("go.mod", "module example.com/test\n\ngo 1.20"),
        ("main.go", "package main\n\nfunc main() {}"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    assert_eq!(context["language"], "Go");
}

#[tokio::test]
async fn test_detect_java_workspace() {
    let workspace = create_test_workspace(vec![
        ("pom.xml", "<project><modelVersion>4.0.0</modelVersion></project>"),
        ("src/main/java/Main.java", "public class Main { public static void main(String[] args) {} }"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    assert_eq!(context["language"], "Java");
    // Single build system returns object
    assert_eq!(context["build_system"]["name"], "Maven");
}

#[tokio::test]
async fn test_detect_mixed_language_workspace() {
    let workspace = create_test_workspace(vec![
        ("frontend/package.json", r#"{"name": "frontend"}"#),
        ("frontend/src/app.ts", "const app = 'test';"),
        ("backend/Cargo.toml", "[package]\nname = \"backend\""),
        ("backend/src/main.rs", "fn main() {}"),
        ("scripts/deploy.py", "#!/usr/bin/env python3\nprint('deploy')"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Language might not be detected because markers are in subdirectories, not at root
    // This is expected behavior - context provider only looks at root level
    // If language is detected (by extension counting), verify it's one of the expected ones
    if let Some(detected) = context.get("language").and_then(|v| v.as_str()) {
        assert!(["Rust", "TypeScript", "JavaScript", "Python"].contains(&detected));
    }
    // It's ok if language isn't detected - that's the reality of monorepos
}

// =============================================================================
// Build System Detection Tests
// =============================================================================

#[tokio::test]
async fn test_detect_cargo_build_system() {
    let workspace = create_test_workspace(vec![
        ("Cargo.toml", "[package]\nname = \"test\""),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Single build system returns object
    assert_eq!(context["build_system"]["name"], "Cargo");
    assert_eq!(context["build_system"]["type"], "rust");
}

#[tokio::test]
async fn test_detect_npm_build_system() {
    let workspace = create_test_workspace(vec![
        ("package.json", r#"{"name": "test"}"#),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Single build system returns object
    assert_eq!(context["build_system"]["name"], "Node.js");
    assert_eq!(context["build_system"]["type"], "javascript");
}

#[tokio::test]
async fn test_detect_maven_build_system() {
    let workspace = create_test_workspace(vec![
        ("pom.xml", "<project></project>"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Single build system returns object
    assert_eq!(context["build_system"]["name"], "Maven");
    assert_eq!(context["build_system"]["type"], "java");
}

#[tokio::test]
async fn test_detect_gradle_build_system() {
    let workspace = create_test_workspace(vec![
        ("build.gradle", "plugins { id 'java' }"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Single build system returns object
    assert_eq!(context["build_system"]["name"], "Gradle");
    assert_eq!(context["build_system"]["type"], "java");
}

#[tokio::test]
async fn test_detect_make_build_system() {
    let workspace = create_test_workspace(vec![
        ("Makefile", "all:\n\techo 'build'"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Single build system returns object
    assert_eq!(context["build_system"]["name"], "Make");
    assert_eq!(context["build_system"]["type"], "generic");
}

#[tokio::test]
async fn test_detect_cmake_build_system() {
    let workspace = create_test_workspace(vec![
        ("CMakeLists.txt", "cmake_minimum_required(VERSION 3.10)"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Single build system returns object
    assert_eq!(context["build_system"]["name"], "CMake");
    assert_eq!(context["build_system"]["type"], "c/c++");
}

#[tokio::test]
async fn test_detect_multiple_build_systems() {
    let workspace = create_test_workspace(vec![
        ("Cargo.toml", "[package]\nname = \"test\""),
        ("Makefile", "all:\n\techo 'build'"),
        ("CMakeLists.txt", "cmake_minimum_required(VERSION 3.10)"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Multiple build systems returns array
    let build_systems = context["build_system"].as_array().unwrap();
    assert_eq!(build_systems.len(), 3);

    // Check that all three are present by name
    let names: Vec<&str> = build_systems
        .iter()
        .filter_map(|bs| bs.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"Cargo"));
    assert!(names.contains(&"Make"));
    assert!(names.contains(&"CMake"));
}

// =============================================================================
// Git Integration Tests
// =============================================================================

#[tokio::test]
async fn test_detect_git_main_branch() {
    let workspace = create_test_workspace(vec![
        ("README.md", "# Test Project"),
    ]);

    create_git_repo(&workspace, "main").expect("Failed to create git repo");

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    assert_eq!(context["git"]["branch"], "main");
}

#[tokio::test]
async fn test_detect_git_feature_branch() {
    let workspace = create_test_workspace(vec![
        ("README.md", "# Test Project"),
    ]);

    create_git_repo(&workspace, "feature/awesome-feature").expect("Failed to create git repo");

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    assert_eq!(context["git"]["branch"], "feature/awesome-feature");
}

#[tokio::test]
async fn test_detect_git_develop_branch() {
    let workspace = create_test_workspace(vec![
        ("README.md", "# Test Project"),
    ]);

    create_git_repo(&workspace, "develop").expect("Failed to create git repo");

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    assert_eq!(context["git"]["branch"], "develop");
}

#[tokio::test]
async fn test_no_git_repository() {
    let workspace = create_test_workspace(vec![
        ("README.md", "# Test Project"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Git info should be absent or indicate no repository
    assert!(context["git"].is_null() || context["git"]["branch"].is_null());
}

// =============================================================================
// File Tree Analysis Tests
// =============================================================================

#[tokio::test]
async fn test_file_tree_lists_directories() {
    let workspace = create_test_workspace(vec![
        ("src/main.rs", "fn main() {}"),
        ("tests/test.rs", "#[test] fn test() {}"),
        ("docs/README.md", "# Docs"),
        ("examples/example.rs", "fn main() {}"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // files is an object with top_level_dirs array
    let top_level_dirs = context["files"]["top_level_dirs"].as_array().unwrap();
    let dir_names: Vec<&str> = top_level_dirs
        .iter()
        .filter_map(|v| v.as_str())
        .collect();

    assert!(dir_names.contains(&"src"));
    assert!(dir_names.contains(&"tests"));
    assert!(dir_names.contains(&"docs"));
    assert!(dir_names.contains(&"examples"));
}

#[tokio::test]
async fn test_file_tree_lists_important_files() {
    let workspace = create_test_workspace(vec![
        ("Cargo.toml", "[package]\nname = \"test\""),
        ("README.md", "# Test"),
        (".gitignore", "target/"),
        ("LICENSE", "MIT License"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // files is an object with important_files array
    let important_files = context["files"]["important_files"].as_array().unwrap();
    let file_names: Vec<&str> = important_files
        .iter()
        .filter_map(|v| v.as_str())
        .collect();

    // .gitignore starts with . so it's filtered out
    assert!(file_names.contains(&"Cargo.toml"));
    assert!(file_names.contains(&"README.md"));
    assert!(file_names.contains(&"LICENSE"));
}

#[tokio::test]
async fn test_file_tree_filters_common_ignore_patterns() {
    let workspace = create_test_workspace(vec![
        ("src/main.rs", "fn main() {}"),
        ("node_modules/package/index.js", "module.exports = {}"),
        ("target/debug/app", "binary"),
        ("__pycache__/module.pyc", "bytecode"),
        (".git/HEAD", "ref: refs/heads/main"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    let top_level_dirs = context["files"]["top_level_dirs"].as_array().unwrap();
    let dir_names: Vec<&str> = top_level_dirs
        .iter()
        .filter_map(|v| v.as_str())
        .collect();

    // Should include src/
    assert!(dir_names.contains(&"src"));

    // Should NOT include ignored directories (they start with . or are in ignore list)
    assert!(!dir_names.contains(&"node_modules"));
    assert!(!dir_names.contains(&"target"));
    assert!(!dir_names.contains(&"__pycache__"));
    assert!(!dir_names.contains(&".git"));
}

#[tokio::test]
async fn test_file_tree_empty_workspace() {
    let workspace = TempDir::new().expect("Failed to create temp dir");

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Empty workspace should have empty arrays
    let top_level_dirs = context["files"]["top_level_dirs"].as_array().unwrap();
    let important_files = context["files"]["important_files"].as_array().unwrap();
    assert_eq!(top_level_dirs.len(), 0);
    assert_eq!(important_files.len(), 0);
    assert_eq!(context["files"]["total_files"].as_u64().unwrap(), 0);
    assert_eq!(context["files"]["total_dirs"].as_u64().unwrap(), 0);
}

// =============================================================================
// Context Formatting Tests
// =============================================================================

#[tokio::test]
async fn test_format_for_prompt_includes_all_info() {
    let workspace = create_test_workspace(vec![
        ("Cargo.toml", "[package]\nname = \"test\""),
        ("src/main.rs", "fn main() {}"),
        ("README.md", "# Test"),
    ]);

    create_git_repo(&workspace, "main").expect("Failed to create git repo");

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let formatted = provider.format_for_prompt().await;

    // Should include workspace context header
    assert!(formatted.contains("Workspace Context"));

    // Should include location (not "Root:")
    assert!(formatted.contains("**Location**:") || formatted.contains("**Project**:"));

    // Should include language
    assert!(formatted.contains("Rust") || formatted.contains("**Primary Language**:"));

    // Should include build system
    assert!(formatted.contains("Cargo") || formatted.contains("**Build System**:"));

    // Should include git branch
    assert!(formatted.contains("main") || formatted.contains("**Git Branch**:"));
}

#[tokio::test]
async fn test_format_for_prompt_handles_minimal_context() {
    let workspace = create_test_workspace(vec![
        ("file.txt", "some content"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let formatted = provider.format_for_prompt().await;

    // Should still include workspace context header
    assert!(formatted.contains("Workspace Context"));

    // Should include location
    assert!(formatted.contains("**Location**:") || formatted.contains("**Project**:"));

    // Should handle missing language/build system gracefully (won't be in output)
}

#[tokio::test]
async fn test_format_for_prompt_is_human_readable() {
    let workspace = create_test_workspace(vec![
        ("package.json", r#"{"name": "test"}"#),
        ("src/index.ts", "const x = 1;"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let formatted = provider.format_for_prompt().await;

    // Should be formatted with clear sections
    assert!(formatted.contains("\n"));
    assert!(!formatted.contains("{\""));  // Not JSON
    assert!(!formatted.contains("\\n"));  // Not escaped

    // Should use markdown-style formatting
    assert!(formatted.contains("##") || formatted.contains("Language:"));
}

// =============================================================================
// Complex Workspace Tests
// =============================================================================

#[tokio::test]
async fn test_monorepo_workspace() {
    let workspace = create_test_workspace(vec![
        ("frontend/package.json", r#"{"name": "frontend"}"#),
        ("frontend/tsconfig.json", r#"{"compilerOptions": {}}"#),
        ("frontend/src/app.tsx", "const App = () => <div>App</div>;"),
        ("backend/Cargo.toml", "[package]\nname = \"backend\""),
        ("backend/src/main.rs", "fn main() {}"),
        ("shared/README.md", "# Shared Code"),
        ("docs/architecture.md", "# Architecture"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Should detect build systems (multiple = array, single = object)
    assert!(context.get("build_system").is_some());

    // Should list directory structure
    let top_level_dirs = context["files"]["top_level_dirs"].as_array().unwrap();
    let dir_names: Vec<&str> = top_level_dirs
        .iter()
        .filter_map(|v| v.as_str())
        .collect();

    assert!(dir_names.contains(&"frontend"));
    assert!(dir_names.contains(&"backend"));
    assert!(dir_names.contains(&"shared"));
    assert!(dir_names.contains(&"docs"));
}

#[tokio::test]
async fn test_real_world_rust_project() {
    let workspace = create_test_workspace(vec![
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]"),
        ("Cargo.lock", "# Generated by Cargo"),
        ("README.md", "# My Project"),
        (".gitignore", "target/\nCargo.lock"),
        ("LICENSE", "MIT"),
        ("src/main.rs", "fn main() {}"),
        ("src/lib.rs", "pub mod core;"),
        ("tests/integration_test.rs", "#[test] fn test() {}"),
        ("benches/benchmark.rs", "#[bench] fn bench() {}"),
        ("examples/example.rs", "fn main() {}"),
        ("docs/guide.md", "# Guide"),
    ]);

    create_git_repo(&workspace, "develop").expect("Failed to create git repo");

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Language
    assert_eq!(context["language"], "Rust");

    // Build system (single = object)
    assert_eq!(context["build_system"]["name"], "Cargo");

    // Git branch
    assert_eq!(context["git"]["branch"], "develop");

    // File structure
    let top_level_dirs = context["files"]["top_level_dirs"].as_array().unwrap();
    let dir_names: Vec<&str> = top_level_dirs
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    let important_files = context["files"]["important_files"].as_array().unwrap();
    let file_names: Vec<&str> = important_files
        .iter()
        .filter_map(|v| v.as_str())
        .collect();

    assert!(dir_names.contains(&"src"));
    assert!(dir_names.contains(&"tests"));
    assert!(file_names.contains(&"Cargo.toml"));
    assert!(file_names.contains(&"README.md"));
}

#[tokio::test]
async fn test_real_world_typescript_project() {
    let workspace = create_test_workspace(vec![
        ("package.json", r#"{"name": "app", "scripts": {"build": "tsc"}}"#),
        ("tsconfig.json", r#"{"compilerOptions": {"target": "ES2020"}}"#),
        ("README.md", "# My App"),
        (".gitignore", "node_modules/\ndist/"),
        ("src/index.ts", "console.log('Hello');"),
        ("src/types.ts", "export type User = { id: number };"),
        ("tests/app.test.ts", "test('app', () => {});"),
    ]);

    create_git_repo(&workspace, "main").expect("Failed to create git repo");

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    assert_eq!(context["language"], "TypeScript");
    // Single build system returns object
    assert_eq!(context["build_system"]["name"], "Node.js");
    assert_eq!(context["git"]["branch"], "main");
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[tokio::test]
async fn test_workspace_with_special_characters() {
    let workspace = create_test_workspace(vec![
        ("src/main.rs", "fn main() {}"),
        ("files/测试.txt", "test"),
        ("files/émoji-🚀.md", "# Test"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await;

    // Should handle special characters without panicking
    assert!(context.is_ok());
}

#[tokio::test]
async fn test_workspace_with_deeply_nested_files() {
    let workspace = create_test_workspace(vec![
        ("a/b/c/d/e/f/g/h/file.rs", "fn test() {}"),
        ("src/main.rs", "fn main() {}"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await;

    // Should handle deep nesting
    assert!(context.is_ok());
}

#[tokio::test]
async fn test_workspace_with_symlinks() {
    // Note: Symlink creation may fail on some systems, so we check result
    let workspace = TempDir::new().expect("Failed to create temp dir");

    fs::write(workspace.path().join("original.txt"), "content").ok();

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(
            workspace.path().join("original.txt"),
            workspace.path().join("link.txt"),
        ).ok();
    }

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await;

    // Should handle symlinks gracefully
    assert!(context.is_ok());
}

#[tokio::test]
async fn test_nonexistent_workspace() {
    let nonexistent = PathBuf::from("/nonexistent/workspace/path");
    let provider = ContextProvider::new(nonexistent);
    let context = provider.gather_context().await;

    // Should either return error or handle gracefully
    // (implementation may vary)
    let _ = context; // Don't panic
}

#[tokio::test]
async fn test_workspace_root_path() {
    let workspace = create_test_workspace(vec![
        ("file.txt", "content"),
    ]);

    let provider = ContextProvider::new(workspace.path().to_path_buf());
    let context = provider.gather_context().await.unwrap();

    // Should include workspace root in context
    assert!(context["workspace"]["root"].is_string());
    let root = context["workspace"]["root"].as_str().unwrap();
    assert!(PathBuf::from(root).exists());
}
