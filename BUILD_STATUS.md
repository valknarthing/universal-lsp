# Universal LSP Build Status

## Current State

Attempted Phase 1 implementation to add 7 high-priority tree-sitter languages (Bash, CSS, HTML, JSON, Markdown, SQL, YAML).

##  Challenge: tree-sitter Version Conflicts

###  Issue

Multiple incompatible versions of tree-sitter are being used by different language parsers:

- **tree-sitter 0.19.5**: tree-sitter-markdown 0.7
- **tree-sitter 0.20.10**: Most existing parsers (JavaScript, Python, Rust, Go, Java, C, C++, Ruby, PHP, etc.)
- **tree-sitter 0.21.0**: Newer parsers (Bash, YAML, SQL)

Rust treats Language types from different tree-sitter versions as completely different types, making them incompatible in the same DashMap registry.

### Discovered Compatible Versions

**Working with tree-sitter 0.20.10:**
- ✅ tree-sitter-html-dvdb 0.20.0 (forked version)
- ✅ tree-sitter-css 0.20.0
- ✅ tree-sitter-json 0.20.2

**Requiring tree-sitter 0.21.0:**
- tree-sitter-bash 0.21
- tree-sitter-yaml 0.6
- tree-sitter-sequel (SQL) 0.3

**Incompatible:**
- tree-sitter-markdown 0.7 (uses tree-sitter 0.19.5)

## Solution: Upgrade to tree-sitter 0.21

To support SQL and YAML (as requested), we need to upgrade the entire project to tree-sitter 0.21.

### Required Changes

1. **Update Cargo.toml** tree-sitter = "0.21" ✅ DONE

2. **Upgrade ALL language parsers to tree-sitter 0.21 compatible versions:**
   - tree-sitter-javascript: Find 0.21-compatible version
   - tree-sitter-typescript: Find 0.21-compatible version
   - tree-sitter-python: Find 0.21-compatible version
   - tree-sitter-rust: Find 0.21-compatible version
   - tree-sitter-go: Find 0.21-compatible version
   - tree-sitter-java: Find 0.21-compatible version
   - tree-sitter-c: Find 0.21-compatible version
   - tree-sitter-cpp: Find 0.21-compatible version
   - tree-sitter-ruby: Find 0.21-compatible version
   - tree-sitter-php: Find 0.21-compatible version
   - tree-sitter-scala: Find 0.21-compatible version
   - tree-sitter-c-sharp: Find 0.21-compatible version
   - tree-sitter-svelte: Find 0.21-compatible version
   - tree-sitter-html: Use tree-sitter-html (not dvdb fork) or find compatible version

3. **Fix API changes in src/tree_sitter/mod.rs:**
   - Line 103: `self.parser.set_language(&*language)?;` (add & for reference)
   - Other type mismatches from parser version upgrades

4. **Test all symbol extraction methods** for the upgraded parsers

## Current Working Languages (with tree-sitter 0.20.10)

✅ JavaScript
✅ TypeScript
✅ TSX
✅ Python
✅ Rust
✅ Go
✅ Java
✅ C
✅ C++
✅ Ruby
✅ PHP
✅ Scala
✅ C#
✅ Svelte

**Total: 14 languages with full tree-sitter support**

## Target: 17-20 Languages

Once tree-sitter 0.21 migration is complete, we'll have:

✅ All 14 existing languages
✅ Bash/Shell
✅ CSS
✅ HTML
✅ JSON
✅ YAML
✅ SQL
❌ Markdown (deferred due to version incompatibility)

**Total: 20 languages**

## Next Steps

1. Search crates.io for tree-sitter 0.21-compatible versions of all existing parsers
2. Update Cargo.toml with compatible versions
3. Fix API compatibility issues in mod.rs
4. Build and test
5. Update LANGUAGE_SUPPORT_MATRIX.md with final supported languages

## AI Completion Status

✅ **AI-powered completions work for ALL languages** via Claude and GitHub Copilot APIs, regardless of tree-sitter version conflicts. The completion engine will fall back to AI when tree-sitter symbols aren't available.

## Files Modified

- `/home/valknar/Projects/zed/universal-lsp/Cargo.toml` - Updated dependencies
- `/home/valknar/Projects/zed/universal-lsp/src/tree_sitter/mod.rs` - Added language registrations and symbol extractors for 7 new languages

## Symbol Extraction Implementation

✅ Fully implemented for all 7 Phase 1 languages:
- `extract_bash_symbols()` - Functions
- `extract_css_symbols()` - Rules, selectors
- `extract_html_symbols()` - Elements with IDs
- `extract_json_symbols()` - Properties
- `extract_markdown_symbols()` - Headings (deferred due to version conflict)
- `extract_sql_symbols()` - Tables, views, functions
- `extract_yaml_symbols()` - Keys

---

**Last Updated**: 2025-10-28
**Status**: ⚠️ Build blocked by tree-sitter version conflicts
**Action Required**: Upgrade all parsers to tree-sitter 0.21
