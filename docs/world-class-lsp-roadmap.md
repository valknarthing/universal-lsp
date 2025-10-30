# World-Class LSP Roadmap 🚀

## Vision
Build Universal LSP into the most advanced, AI-powered language server with rich coding support across 19+ languages, combining tree-sitter parsing, Claude AI, and MCP ecosystem integration.

## Current Status ✅

### What We Have (Tier 1 - Basic LSP)
- [x] **Hover**: Enhanced with docstrings, function signatures, parameters
- [x] **Completion**: AI-powered (Claude) + tree-sitter symbols
- [x] **Go-to-Definition**: Working across all languages
- [x] **Find References**: Implemented
- [x] **Document Symbols**: Full tree-sitter extraction
- [x] **AI Integration**: Claude completions active
- [x] **Multi-Language**: 19 languages supported
- [x] **220/220 Tests Passing**: Full test coverage

### What's Missing (Tier 2-5)
Everything else that makes an LSP world-class!

---

## Implementation Roadmap

### 🎯 **Phase 1: Rich Diagnostics** (Immediate Priority)
**Impact**: ⭐⭐⭐⭐⭐ (Highest value for developers)
**Effort**: Medium
**Timeline**: 2-3 hours

#### Features to Implement:
1. **Real-time Error Detection**
   - Parse errors from tree-sitter error nodes
   - Syntax errors highlighted in red
   - Published via `textDocument/publishDiagnostics`

2. **Semantic Diagnostics**
   - Undefined variables
   - Type mismatches (where tree-sitter provides type info)
   - Unused imports/variables
   - Style violations

3. **AI-Enhanced Diagnostics**
   - Claude analyzes code for potential issues
   - Suggests improvements
   - Detects code smells

**Implementation**:
```rust
// src/diagnostics/mod.rs (new module)
pub async fn compute_diagnostics(
    tree: &Tree,
    source: &str,
    lang: &str,
    claude_client: Option<&ClaudeClient>
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // 1. Tree-sitter syntax errors
    diagnostics.extend(extract_syntax_errors(tree, source));

    // 2. Semantic errors
    diagnostics.extend(check_undefined_symbols(tree, source, lang));

    // 3. AI-enhanced analysis (optional)
    if let Some(claude) = claude_client {
        diagnostics.extend(claude_analyze(source, lang).await);
    }

    diagnostics
}
```

**Expected Result**:
- Real-time error squiggles in editor
- Warning for undefined variables
- AI suggestions for improvements

---

### 🎯 **Phase 2: Code Actions** (High Priority)
**Impact**: ⭐⭐⭐⭐⭐ (Critical for productivity)
**Effort**: Medium-High
**Timeline**: 3-4 hours

#### Features to Implement:
1. **Quick Fixes**
   - Fix undefined variable (import/define)
   - Fix type errors
   - Remove unused code
   - Add missing imports

2. **Refactorings**
   - Extract variable
   - Extract function
   - Inline variable
   - Rename symbol (enhanced)
   - Convert to const/let (JS)
   - Add type annotations (Python, TS)

3. **AI-Powered Actions**
   - "Explain this code" (Claude analysis)
   - "Optimize this function"
   - "Add error handling"
   - "Write tests for this"
   - "Document this code"

**Implementation**:
```rust
// src/code_actions/mod.rs (new module)
pub async fn compute_code_actions(
    tree: &Tree,
    source: &str,
    range: Range,
    diagnostics: &[Diagnostic],
    claude_client: Option<&ClaudeClient>
) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Quick fixes for diagnostics
    for diagnostic in diagnostics {
        if let Some(fix) = suggest_quick_fix(diagnostic, tree, source) {
            actions.push(fix);
        }
    }

    // Refactoring actions
    if can_extract_variable(range, tree, source) {
        actions.push(create_extract_variable_action(range));
    }

    // AI-powered actions
    if let Some(claude) = claude_client {
        actions.push(create_explain_action(claude));
        actions.push(create_optimize_action(claude));
    }

    actions
}
```

**Expected Result**:
- Light bulb icon appears on errors
- Right-click menu shows "Extract variable", "Add import", etc.
- AI actions: "Explain with Claude", "Optimize with AI"

---

### 🎯 **Phase 3: Signature Help** (High Priority)
**Impact**: ⭐⭐⭐⭐ (Very helpful while typing)
**Effort**: Low-Medium
**Timeline**: 1-2 hours

#### Features to Implement:
1. **Parameter Info While Typing**
   - Show function signature when typing arguments
   - Highlight current parameter
   - Show parameter types and descriptions

2. **Overload Support**
   - Show all function overloads
   - Navigate between overloads
   - Highlight best match

**Implementation**:
```rust
// In src/main.rs
async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    if let Some(content) = self.documents.get(uri.as_str()) {
        let lang = detect_language(uri.path());

        // Find function call at position
        if let Some(call_info) = find_function_call_at_position(&content, position, &lang) {
            return Ok(Some(SignatureHelp {
                signatures: vec![SignatureInformation {
                    label: call_info.signature,
                    documentation: call_info.documentation,
                    parameters: call_info.parameters.into_iter().map(|p| {
                        ParameterInformation {
                            label: ParameterLabel::Simple(p.name),
                            documentation: Some(Documentation::String(p.doc)),
                        }
                    }).collect(),
                    active_parameter: Some(call_info.active_param_index),
                }],
                active_signature: Some(0),
                active_parameter: Some(call_info.active_param_index),
            }));
        }
    }

    Ok(None)
}
```

**Expected Result**:
- Tooltip shows parameter info while typing function calls
- Current parameter is highlighted
- Shows parameter types and descriptions

---

### 🎯 **Phase 4: MCP Integration** (Game Changer!)
**Impact**: ⭐⭐⭐⭐⭐ (Unique differentiator)
**Effort**: Medium
**Timeline**: 2-3 hours

#### Features to Enable:
1. **Filesystem MCP**
   - Read project files for context
   - Suggest imports from available modules
   - Show file structure in hover

2. **Git MCP**
   - Show git blame in hover
   - Suggest based on recent commits
   - Show file history context

3. **Web Search MCP**
   - Search documentation while coding
   - Show MDN/Python docs in hover
   - Suggest based on Stack Overflow

4. **Database MCP** (PostgreSQL, SQLite)
   - Query completion for SQL
   - Show table schemas
   - Validate SQL syntax

**Implementation Strategy**:
```rust
// Update Zed extension to pass MCP config
// In universal/src/lib.rs
let mcp_args = vec![
    "--mcp-server".to_string(),
    format!("filesystem=npx -y @modelcontextprotocol/server-filesystem {}", workspace_root),
    "--mcp-server".to_string(),
    "git=npx -y @modelcontextprotocol/server-git".to_string(),
    "--mcp-server".to_string(),
    "brave-search=npx -y @modelcontextprotocol/server-brave-search".to_string(),
];

// Merge into args
args.extend(mcp_args);
```

**Alternative**: Implement TOML config loading to avoid cluttering extension

**Expected Result**:
- Hover shows git blame and file context
- Completions include imports from all project files
- Documentation appears inline from web search

---

### 🎯 **Phase 5: Semantic Tokens** (Visual Enhancement)
**Impact**: ⭐⭐⭐⭐ (Better syntax highlighting)
**Effort**: Medium
**Timeline**: 2-3 hours

#### Features to Implement:
1. **Token Classification**
   - Variables vs. functions vs. classes
   - Mutable vs. immutable
   - Deprecated symbols
   - Unused code (grayed out)

2. **Semantic Colors**
   - Different colors for different token types
   - Highlight important symbols
   - Dim unused imports

**Implementation**:
```rust
async fn semantic_tokens_full(
    &self,
    params: SemanticTokensParams
) -> Result<Option<SemanticTokensResult>> {
    let uri = &params.text_document.uri;

    if let Some(content) = self.documents.get(uri.as_str()) {
        let lang = detect_language(uri.path());
        let mut parser = TreeSitterParser::new()?;
        parser.set_language(&lang)?;
        let tree = parser.parse(&content, uri.as_str())?;

        // Extract semantic tokens
        let tokens = extract_semantic_tokens(&tree, &content, &lang);

        return Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })));
    }

    Ok(None)
}
```

**Expected Result**:
- Better syntax highlighting (semantic vs. syntactic)
- Unused variables grayed out
- Mutable variables highlighted differently

---

### 🎯 **Phase 6: Inlay Hints** (Modern Editor Feature)
**Impact**: ⭐⭐⭐⭐ (Very helpful for understanding code)
**Effort**: Medium
**Timeline**: 2-3 hours

#### Features to Implement:
1. **Type Hints**
   - Show inferred types inline
   - Parameter types in function calls
   - Return types for functions

2. **Parameter Name Hints**
   - Show parameter names in function calls
   - Especially helpful for boolean parameters

**Implementation**:
```rust
async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
    let uri = &params.text_document.uri;
    let range = params.range;

    if let Some(content) = self.documents.get(uri.as_str()) {
        let lang = detect_language(uri.path());
        let hints = compute_inlay_hints(&content, range, &lang);
        return Ok(Some(hints));
    }

    Ok(None)
}

fn compute_inlay_hints(source: &str, range: Range, lang: &str) -> Vec<InlayHint> {
    let mut hints = Vec::new();

    // Find function calls and add parameter name hints
    // Find variable declarations and add type hints

    hints
}
```

**Expected Result**:
- Type annotations appear inline: `let count/* : i32 */ = 42;`
- Parameter names in calls: `calculate(/* x: */ 10, /* y: */ 20)`

---

### 🎯 **Phase 7: Document Formatting** (Polish)
**Impact**: ⭐⭐⭐ (Nice to have)
**Effort**: Low-Medium
**Timeline**: 1-2 hours

#### Features to Implement:
1. **Format Entire Document**
   - Use language-specific formatters
   - Python: black, autopep8
   - JavaScript: prettier
   - Rust: rustfmt

2. **Format Selection**
   - Format only selected code
   - Preserve surrounding code

3. **Format on Save**
   - Automatic formatting
   - Configurable

**Implementation**:
```rust
async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
    let uri = &params.text_document.uri;
    let lang = detect_language(uri.path());

    if let Some(content) = self.documents.get(uri.as_str()) {
        // Delegate to language-specific formatter
        let formatted = format_code(&content, &lang, &params.options)?;

        // Return single edit replacing entire document
        return Ok(Some(vec![TextEdit {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position {
                    line: content.lines().count() as u32,
                    character: 0
                },
            },
            new_text: formatted,
        }]));
    }

    Ok(None)
}
```

---

### 🎯 **Phase 8: Code Lens** (Advanced Feature)
**Impact**: ⭐⭐⭐ (Helpful for navigation)
**Effort**: Medium
**Timeline**: 2-3 hours

#### Features to Implement:
1. **Reference Count**
   - Show "5 references" above functions/classes
   - Click to show all references

2. **Run/Debug**
   - "Run test" above test functions
   - "Debug function" above main functions

3. **Git Lens**
   - Show last commit info above functions
   - "Modified 2 days ago by X"

**Expected Result**:
- Inline annotations above symbols
- Quick actions without opening menus

---

## Implementation Priority

### **Sprint 1: Foundation** (6-8 hours)
1. ✅ Real-time Diagnostics
2. ✅ Code Actions (Quick Fixes)
3. ✅ Signature Help

**Goal**: Make Universal LSP feel responsive and helpful

### **Sprint 2: Intelligence** (6-8 hours)
4. ✅ MCP Integration
5. ✅ AI-Enhanced Code Actions
6. ✅ Semantic Tokens

**Goal**: Add unique AI-powered features

### **Sprint 3: Polish** (4-6 hours)
7. ✅ Inlay Hints
8. ✅ Document Formatting
9. ✅ Code Lens

**Goal**: Complete the world-class experience

---

## Success Metrics

### Developer Experience
- [ ] Errors appear **immediately** while typing
- [ ] Code actions suggest **intelligent** fixes
- [ ] AI completions are **contextually relevant**
- [ ] MCP provides **rich context** from multiple sources
- [ ] Formatting works **seamlessly**

### Performance
- [ ] Diagnostics computed in <100ms
- [ ] Completions appear in <500ms (tree-sitter) or <2s (AI)
- [ ] No UI freezing
- [ ] Handles files up to 10,000 lines

### Feature Completeness
- [ ] All LSP protocol features implemented
- [ ] Unique AI-powered features
- [ ] MCP integration working
- [ ] Multi-language support (19+)

---

## Technical Architecture

### Module Structure
```
src/
├── main.rs (LSP server, handlers)
├── diagnostics/ (NEW)
│   ├── mod.rs (main diagnostics logic)
│   ├── syntax.rs (tree-sitter errors)
│   ├── semantic.rs (undefined symbols, etc.)
│   └── ai.rs (Claude-enhanced diagnostics)
├── code_actions/ (NEW)
│   ├── mod.rs (code action computation)
│   ├── quick_fixes.rs (auto-fixes for diagnostics)
│   ├── refactorings.rs (extract, inline, rename)
│   └── ai_actions.rs (Claude-powered actions)
├── signature_help/ (NEW)
│   └── mod.rs (signature help computation)
├── semantic_tokens/ (NEW)
│   └── mod.rs (token classification)
├── inlay_hints/ (NEW)
│   └── mod.rs (type and parameter hints)
├── formatting/ (NEW)
│   └── mod.rs (document formatting)
└── (existing modules...)
```

---

## Next Steps

1. **Create diagnostics module** (start here!)
2. **Update ServerCapabilities** to advertise new features
3. **Add handlers to main.rs**
4. **Write tests for each feature**
5. **Update documentation**
6. **Release v0.2.0** with "World-Class LSP" tag

---

## The Vision

Imagine a developer using Universal LSP:

1. **Types a function call** → Signature help shows parameters with AI-suggested examples
2. **Makes a typo** → Real-time diagnostic highlights error, suggests fix
3. **Clicks lightbulb** → AI offers to explain, optimize, or fix the code
4. **Hovers over import** → Sees git blame, file structure, and web docs inline
5. **Requests completion** → Gets AI-generated code based on context from MCP servers
6. **Saves file** → Automatic formatting with best practices

**This is the world-class LSP we're building!** 🚀

Let's start with diagnostics - the highest impact feature! 💪
