# Universal LSP Language Support Matrix

## Overview

This document compares language support between:
- **Zed Editor** built-in language extensions
- **Universal LSP** tree-sitter grammar support

## Currently Supported Languages (with Tree-sitter)

These languages have full tree-sitter grammar support in Universal LSP (src/tree_sitter/mod.rs):

| Language | Zed Extension | Tree-sitter Parser | Symbol Extraction | Status |
|----------|--------------|-------------------|-------------------|--------|
| JavaScript | ✅ | ✅ `tree-sitter-javascript` | ✅ Functions, Classes | **FULL** |
| TypeScript | ✅ | ✅ `tree-sitter-typescript` | ✅ Functions, Classes | **FULL** |
| TSX | ✅ | ✅ `tree-sitter-typescript` | ✅ Functions, Classes | **FULL** |
| Python | ✅ | ✅ `tree-sitter-python` | ✅ Functions, Classes | **FULL** |
| Rust | ✅ | ✅ `tree-sitter-rust` | ✅ Functions, Structs, Enums | **FULL** |
| Go | ✅ | ✅ `tree-sitter-go` | ✅ Functions, Types | **FULL** |
| Java | ✅ | ✅ `tree-sitter-java` | ✅ Methods, Classes | **FULL** |
| C | ⚠️ (needs check) | ✅ `tree-sitter-c` | ✅ Functions | **FULL** |
| C++ | ⚠️ (needs check) | ✅ `tree-sitter-cpp` | ✅ Functions | **FULL** |
| Svelte | ✅ | ✅ `tree-sitter-svelte` | ✅ Functions, Variables | **FULL** |

**Total: 10 languages with full tree-sitter support**

## Zed Languages WITHOUT Tree-sitter in Universal LSP

These languages have Zed extensions but NO tree-sitter parser in Universal LSP yet:

| Language | Zed Extension | Tree-sitter Available? | Priority | Notes |
|----------|--------------|----------------------|----------|-------|
| Angular | ✅ | ✅ (tree-sitter-angular) | 🔴 HIGH | Popular framework |
| Ansible | ✅ | ❌ | 🟡 MEDIUM | YAML-based |
| Astro | ✅ | ✅ (tree-sitter-astro) | 🟡 MEDIUM | Web framework |
| Bash/Shell | ⚠️ (needs check) | ✅ (tree-sitter-bash) | 🔴 HIGH | Very common |
| Clojure | ✅ | ✅ (tree-sitter-clojure) | 🟢 LOW | Niche |
| CSS | ⚠️ (needs check) | ✅ (tree-sitter-css) | 🔴 HIGH | Essential web |
| Dart | ✅ | ✅ (tree-sitter-dart) | 🟡 MEDIUM | Flutter dev |
| Dockerfile | ✅ | ✅ (tree-sitter-dockerfile) | 🟡 MEDIUM | DevOps |
| Elixir | ✅ | ✅ (tree-sitter-elixir) | 🟡 MEDIUM | Phoenix framework |
| Elm | ✅ | ✅ (tree-sitter-elm) | 🟢 LOW | Niche |
| Erlang | ✅ | ✅ (tree-sitter-erlang) | 🟡 MEDIUM | Backend systems |
| Fortran | ✅ | ❌ | 🟢 LOW | Scientific computing |
| GraphQL | ⚠️ (needs check) | ✅ (tree-sitter-graphql) | 🟡 MEDIUM | API development |
| Haskell | ✅ | ✅ (tree-sitter-haskell) | 🟡 MEDIUM | Functional programming |
| HTML | ⚠️ (needs check) | ✅ (tree-sitter-html) | 🔴 HIGH | Essential web |
| JSON | ⚠️ (needs check) | ✅ (tree-sitter-json) | 🔴 HIGH | Universal data format |
| Julia | ✅ | ✅ (tree-sitter-julia) | 🟡 MEDIUM | Scientific computing |
| Kotlin | ✅ | ✅ (tree-sitter-kotlin) | 🟡 MEDIUM | Android dev |
| Lua | ✅ | ✅ (tree-sitter-lua) | 🟡 MEDIUM | Game dev, Neovim |
| Markdown | ⚠️ (needs check) | ✅ (tree-sitter-markdown) | 🔴 HIGH | Documentation |
| Nix | ✅ | ✅ (tree-sitter-nix) | 🟡 MEDIUM | Package management |
| OCaml | ✅ | ✅ (tree-sitter-ocaml) | 🟢 LOW | Functional programming |
| Pascal | ✅ | ❌ | 🟢 LOW | Legacy |
| Perl | ✅ | ✅ (tree-sitter-perl) | 🟡 MEDIUM | Legacy scripts |
| PHP | ✅ | ✅ (tree-sitter-php) | 🟡 MEDIUM | Web backend |
| Prisma | ✅ | ✅ (tree-sitter-prisma) | 🟡 MEDIUM | Database ORM |
| R | ✅ | ✅ (tree-sitter-r) | 🟡 MEDIUM | Data science |
| Ruby | ✅ | ✅ (tree-sitter-ruby) | 🟡 MEDIUM | Rails development |
| Scheme | ✅ | ✅ (tree-sitter-scheme) | 🟢 LOW | Lisp dialect |
| SCSS/Sass | ✅ | ✅ (tree-sitter-scss) | 🟡 MEDIUM | CSS preprocessing |
| SQL | ✅ | ✅ (tree-sitter-sql) | 🔴 HIGH | Database queries |
| Swift | ✅ | ✅ (tree-sitter-swift) | 🟡 MEDIUM | iOS development |
| Terraform | ✅ | ✅ (tree-sitter-hcl) | 🟡 MEDIUM | Infrastructure as code |
| TOML | ✅ | ✅ (tree-sitter-toml) | 🟡 MEDIUM | Config format |
| Vue | ✅ | ✅ (tree-sitter-vue) | 🟡 MEDIUM | Web framework |
| YAML | ⚠️ (needs check) | ✅ (tree-sitter-yaml) | 🔴 HIGH | Config format |
| Zig | ✅ | ✅ (tree-sitter-zig) | 🟡 MEDIUM | Systems programming |

**Total: ~37 additional languages available**

## AI-Powered Completion Support

Universal LSP provides AI-powered completions for **ALL languages** via:

✅ **Claude API** (claude-sonnet-4-20250514)
- Context-aware intelligent suggestions
- Multi-language support
- Configured via `ANTHROPIC_API_KEY` environment variable

✅ **GitHub Copilot API**
- Professional code completions
- Trained on vast codebases
- Configured via `GITHUB_TOKEN` environment variable

**Multi-tier Completion Strategy:**
1. **Tier 0 (Highest Priority)**: AI providers (Claude + Copilot)
2. **Tier 1**: Tree-sitter symbol extraction (10 languages)
3. **Tier 2**: MCP (Model Context Protocol) providers
4. **Tier 3**: Grammar-based keywords (future: all 242 languages)

## Features by Language Category

### Full Support (10 languages)
- ✅ AI-powered completions (Claude + Copilot)
- ✅ Tree-sitter symbol extraction
- ✅ Hover information
- ✅ Go to definition
- ✅ Find references
- ✅ Document symbols (outline view)

### Partial Support (37+ languages with Zed extensions)
- ✅ AI-powered completions (Claude + Copilot)
- ⚠️ No tree-sitter symbols (yet)
- ⚠️ Limited hover/definition support
- ✅ Syntax highlighting (via Zed's grammars)

### Universal Support (242 languages planned)
- ✅ AI-powered completions (Claude + Copilot)
- ⚠️ Grammar-based keywords (planned)
- ⚠️ TextMate grammar integration (planned)
- ✅ Basic language detection

## Priority Recommendations for Next Implementation

### 🔴 HIGH PRIORITY (Essential for web/backend development)
1. **Bash/Shell** - Extremely common for DevOps
2. **CSS** - Essential for web development
3. **HTML** - Essential for web development
4. **JSON** - Universal data format
5. **Markdown** - Documentation everywhere
6. **SQL** - Database queries
7. **YAML** - Config files, CI/CD

### 🟡 MEDIUM PRIORITY (Popular frameworks/languages)
8. **Ruby** - Rails development
9. **PHP** - Web backend (still widely used)
10. **Dart** - Flutter mobile development
11. **Kotlin** - Android development
12. **Swift** - iOS development
13. **Vue** - Popular web framework
14. **Angular** - Enterprise web framework

### 🟢 LOW PRIORITY (Niche/legacy)
- Scheme, Elm, OCaml, Pascal, Fortran

## Implementation Plan

### Phase 1: High-Priority Languages (Weeks 1-2)
Add tree-sitter parsers and symbol extractors for:
- Bash, CSS, HTML, JSON, Markdown, SQL, YAML

**Cargo.toml additions:**
```toml
tree-sitter-bash = "0.23"
tree-sitter-css = "0.23"
tree-sitter-html = "0.23"
tree-sitter-json = "0.23"
tree-sitter-markdown = "0.3"
tree-sitter-sql = "0.3"
tree-sitter-yaml = "0.1"
```

### Phase 2: Medium-Priority Languages (Weeks 3-4)
Add parsers for popular frameworks:
- Ruby, PHP, Dart, Kotlin, Swift, Vue, Angular

### Phase 3: Grammar-based Completion (Week 5+)
Implement TextMate grammar keyword extraction for all 242 languages

## Performance Considerations

### Current Binary Size
- **With 10 parsers**: ~18MB (release build)
- **Estimated with 47 parsers**: ~40-50MB
- **With all 242 grammars embedded**: ~60-80MB

### Memory Usage
- **Current (10 languages)**: ~50-100MB
- **With 47 parsers**: ~150-200MB
- **Lazy loading**: Keep memory under 200MB

### Recommendations
1. ✅ **Keep top 10-15 parsers embedded**
2. ✅ **Lazy-load less common parsers**
3. ✅ **Use on-demand grammar compilation**
4. ✅ **Implement LRU cache for parsed trees**

## Testing Plan

### Integration Tests Needed
- [ ] Symbol extraction for each new language
- [ ] Hover information accuracy
- [ ] Go-to-definition functionality
- [ ] Find references completeness
- [ ] AI completion integration for all languages
- [ ] Performance benchmarks (completion latency <100ms)

### Demo Files Required
Create test files for each newly supported language to verify:
- Syntax highlighting
- Symbol extraction
- Hover information
- AI completions

## References

- **Tree-sitter Registry**: https://github.com/tree-sitter
- **Zed Extensions**: /home/valknar/Projects/zed/extensions/extensions/
- **Universal LSP Source**: src/tree_sitter/mod.rs
- **Cargo Dependencies**: https://crates.io/search?q=tree-sitter-

---

**Last Updated**: 2025-10-28
**Status**: ✅ 10 languages fully supported, 37+ languages with AI completion only
**Next Goal**: Add 7 high-priority parsers (Bash, CSS, HTML, JSON, Markdown, SQL, YAML)
