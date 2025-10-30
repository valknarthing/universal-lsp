//! Universal LSP Server - Main Entry Point
//! Supports 19+ programming languages with AI-powered intelligent code analysis
//!
//! Multi-command CLI:
//! - `ulsp` or `ulsp lsp` - Start LSP server (default mode)
//! - `ulsp acp` - Start ACP agent process
//! - `ulsp zed init` - Initialize Zed workspace with perfect configuration

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use std::sync::Arc;

mod ai;
mod code_actions;
mod config;
mod diagnostics;
mod formatting;
mod language;
mod mcp;
mod pipeline;
mod proxy;
mod semantic_tokens;
mod inlay_hints;
mod signature_help;
mod text_sync;
mod tree_sitter;
mod workspace;
mod coordinator;

use ai::{ClaudeClient, ClaudeConfig, CopilotClient, CopilotConfig, CompletionContext};
use code_actions::CodeActionProvider;
use config::{Config, CommandMode};
use coordinator::CoordinatorClient;
use diagnostics::DiagnosticProvider;
use formatting::FormattingProvider;
use inlay_hints::InlayHintsProvider;
use language::detect_language;
use mcp::McpRequest;
use pipeline::{McpPipeline, merge_mcp_responses, lsp_position_to_mcp};
use proxy::{ProxyConfig, ProxyManager};
use semantic_tokens::SemanticTokensProvider;
use signature_help::SignatureHelpProvider;
use text_sync::TextSyncManager;
use tree_sitter::TreeSitterParser;
use workspace::WorkspaceManager;

struct UniversalLsp {
    client: Client,
    config: Arc<Config>,
    coordinator_client: Option<Arc<CoordinatorClient>>,
    pipeline: Option<Arc<McpPipeline>>,
    proxy_manager: Option<Arc<ProxyManager>>,
    claude_client: Option<Arc<ClaudeClient>>,
    copilot_client: Option<Arc<CopilotClient>>,
    parser: Arc<dashmap::DashMap<String, TreeSitterParser>>,
    documents: Arc<dashmap::DashMap<String, String>>,
    diagnostic_provider: Arc<DiagnosticProvider>,
    code_action_provider: Arc<CodeActionProvider>,
    formatting_provider: Arc<FormattingProvider>,
    semantic_tokens_provider: Arc<SemanticTokensProvider>,
    signature_help_provider: Arc<SignatureHelpProvider>,
    inlay_hints_provider: Arc<InlayHintsProvider>,
    workspace_manager: Arc<WorkspaceManager>,
    text_sync_manager: Arc<TextSyncManager>,
}

impl UniversalLsp {
    fn new(client: Client, config: Config) -> Self {
        // Create MCP pipeline if configured
        let pipeline = if config.has_mcp_pipeline() {
            Some(Arc::new(McpPipeline::new(&config)))
        } else {
            None
        };

        // Create proxy manager if proxies configured
        let proxy_manager = if config.has_proxy_servers() {
            let proxy_configs: std::collections::HashMap<String, ProxyConfig> = config
                .proxy
                .servers
                .iter()
                .filter_map(|(lang, cmd)| {
                    ProxyConfig::from_string(&format!("{}={}", lang, cmd))
                        .map(|pc| (lang.clone(), pc))
                })
                .collect();

            Some(Arc::new(ProxyManager::new(proxy_configs)))
        } else {
            None
        };

        // Create Claude client if API key is available
        let claude_client = if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            let claude_config = ClaudeConfig {
                api_key,
                ..Default::default()
            };
            match ClaudeClient::new(claude_config) {
                Ok(client) => Some(Arc::new(client)),
                Err(e) => {
                    tracing::warn!("Failed to initialize Claude client: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Create Copilot client if API key is available
        let copilot_client = if let Ok(api_key) = std::env::var("GITHUB_TOKEN") {
            let copilot_config = CopilotConfig {
                api_key,
                ..Default::default()
            };
            match CopilotClient::new(copilot_config) {
                Ok(client) => Some(Arc::new(client)),
                Err(e) => {
                    tracing::warn!("Failed to initialize Copilot client: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Try to connect to MCP Coordinator daemon (optional, graceful fallback)
        let coordinator_client = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                match CoordinatorClient::connect().await {
                    Ok(client) => {
                        tracing::info!("Connected to MCP Coordinator daemon");
                        Some(Arc::new(client))
                    }
                    Err(e) => {
                        tracing::info!("MCP Coordinator not available ({}), continuing without MCP", e);
                        None
                    }
                }
            })
        });

        Self {
            client,
            config: Arc::new(config),
            coordinator_client,
            pipeline,
            proxy_manager,
            claude_client: claude_client.clone(),
            copilot_client,
            parser: Arc::new(dashmap::DashMap::new()),
            documents: Arc::new(dashmap::DashMap::new()),
            diagnostic_provider: Arc::new(DiagnosticProvider::new()),
            code_action_provider: Arc::new(CodeActionProvider::with_claude(claude_client)),
            formatting_provider: Arc::new(FormattingProvider::new()),
            semantic_tokens_provider: Arc::new(SemanticTokensProvider::new()),
            signature_help_provider: Arc::new(SignatureHelpProvider::new()),
            inlay_hints_provider: Arc::new(InlayHintsProvider::new()),
            workspace_manager: Arc::new(WorkspaceManager::new()),
            text_sync_manager: Arc::new(TextSyncManager::new()),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for UniversalLsp {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Initialize workspace folders if provided
        if let Some(folders) = params.workspace_folders {
            for folder in folders {
                if let Err(e) = self.workspace_manager.add_folder(folder) {
                    tracing::warn!("Failed to add workspace folder: {}", e);
                }
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensProvider::legend(),
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                        }
                    )
                ),
                inlay_hint_provider: Some(OneOf::Left(InlayHintServerCapabilities::Options(
                    InlayHintOptions {
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        resolve_provider: Some(false),
                    }
                ))),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "Universal LSP".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Universal LSP initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let lang = detect_language(uri.path());
        let lang_lowercase = lang.to_lowercase(); // Normalize for tree-sitter lookup

        tracing::debug!("Hover requested for {} at {}:{}", uri, position.line, position.character);
        let mut hover_text = format!("Language: {}", lang);

        // Try tree-sitter symbol extraction at cursor position
        if let Some(content) = self.documents.get(uri.as_str()) {
            tracing::debug!("Document found, content length: {}", content.len());

            match TreeSitterParser::new() {
                Ok(mut parser) => {
                    tracing::debug!("TreeSitterParser created successfully");

                    match parser.set_language(&lang_lowercase) {
                        Ok(_) => {
                            tracing::debug!("Language '{}' set successfully", lang);

                            match parser.parse(&content, uri.as_str()) {
                                Ok(tree) => {
                                    tracing::debug!("Parsing successful");

                                    // Find node at position
                                    let byte_offset = position_to_byte(&content, position);
                                    tracing::debug!("Byte offset: {}", byte_offset);

                                    if let Some(node) = tree.root_node().descendant_for_byte_range(byte_offset, byte_offset) {
                                        tracing::debug!("Found node: kind='{}'", node.kind());

                                        // Extract rich hover information
                                        match parser.extract_hover_info(node, &content, &lang_lowercase) {
                                            Ok(rich_info) => {
                                                hover_text = format!("Language: {}\n\n{}", lang, rich_info);
                                            }
                                            Err(e) => {
                                                tracing::debug!("Failed to extract hover info: {:?}", e);
                                                // Fallback to basic info
                                                let node_text = &content[node.byte_range()];
                                                hover_text = format!(
                                                    "{}\n\nSymbol: {}\nType: {}",
                                                    hover_text, node_text, node.kind()
                                                );
                                            }
                                        }
                                    } else {
                                        tracing::debug!("No node found at byte offset {}", byte_offset);
                                    }
                                }
                                Err(e) => {
                                    tracing::debug!("Failed to parse: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::debug!("Failed to set language '{}': {:?}", lang, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Failed to create TreeSitterParser: {:?}", e);
                }
            }
        } else {
            tracing::debug!("Document not found in cache: {}", uri.as_str());
        }

        // Query MCP servers via Coordinator for rich hover information
        if let Some(coordinator) = &self.coordinator_client {
            let mcp_request = McpRequest {
                request_type: "hover".to_string(),
                uri: uri.to_string(),
                position: lsp_position_to_mcp(position.line, position.character),
                context: self.documents.get(uri.as_str()).map(|c| c.value().clone()),
            };

            let mut mcp_sections = Vec::new();

            // Query all configured MCP servers for hover information
            for server_name in self.config.mcp.servers.keys() {
                match coordinator.query(server_name, mcp_request.clone()).await {
                    Ok(response) => {
                        // Collect documentation from MCP server
                        if let Some(doc) = response.documentation {
                            mcp_sections.push(format!("**{}**\n{}", server_name, doc));
                        }

                        // Add suggestions as additional context
                        if !response.suggestions.is_empty() {
                            let suggestions = response.suggestions.join(", ");
                            mcp_sections.push(format!(
                                "**{} - Related Symbols**\n{}",
                                server_name,
                                suggestions
                            ));
                        }
                    }
                    Err(e) => {
                        tracing::debug!("MCP hover query to {} failed: {}", server_name, e);
                    }
                }
            }

            // Aggregate MCP results into hover text
            if !mcp_sections.is_empty() {
                hover_text = format!(
                    "{}\n\n## MCP Server Information\n\n{}",
                    hover_text,
                    mcp_sections.join("\n\n---\n\n")
                );
            }

            // AI Enhancement: Send combined context to Claude/Copilot for enrichment
            use crate::ai::claude::CompletionContext;

            let ai_ctx = CompletionContext {
                language: "documentation".to_string(),
                file_path: params.text_document_position_params.text_document.uri.to_string(),
                prefix: hover_text.clone(),
                suffix: None,
                context: Some(format!(
                    "Enhance this documentation with:\
                    \n- Natural language explanation\
                    \n- Usage examples\
                    \n- Best practices\
                    \n- Common pitfalls\
                    \n- Related concepts"
                )),
            };

            // Try Claude first, then Copilot
            let ai_enhancement = if let Some(ref claude) = self.claude_client {
                match claude.get_completions(&ai_ctx).await {
                    Ok(suggestions) if !suggestions.is_empty() => {
                        Some(suggestions[0].text.clone())
                    }
                    _ => None,
                }
            } else if let Some(ref copilot) = self.copilot_client {
                match copilot.get_completions(&ai_ctx).await {
                    Ok(suggestions) if !suggestions.is_empty() => {
                        Some(suggestions[0].text.clone())
                    }
                    _ => None,
                }
            } else {
                None
            };

            // Add AI-enhanced explanation
            if let Some(enhancement) = ai_enhancement {
                hover_text = format!(
                    "{}\n\n## AI-Enhanced Explanation\n\n{}",
                    hover_text,
                    enhancement
                );
            }
        }

        // Try MCP pre-processing if available (fallback for legacy pipeline)
        if let Some(pipeline) = &self.pipeline {
            if pipeline.has_pre_processing() {
                let mcp_request = McpRequest {
                    request_type: "hover".to_string(),
                    uri: uri.to_string(),
                    position: lsp_position_to_mcp(position.line, position.character),
                    context: None,
                };

                if let Ok(responses) = pipeline.pre_process(mcp_request).await {
                    if !responses.is_empty() {
                        let merged = merge_mcp_responses(responses);
                        if let Some(doc) = merged.documentation {
                            hover_text = format!("{}\n\n## Legacy Pipeline\n\n{}", hover_text, doc);
                        }
                    }
                }
            }
        }

        // Try proxy if available
        if let Some(proxy_manager) = &self.proxy_manager {
            if proxy_manager.has_proxy_for(lang) {
                self.client.log_message(
                    MessageType::INFO,
                    format!("Would proxy hover request to {} server", lang)
                ).await;
            }
        }

        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(hover_text)),
            range: None,
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let lang = detect_language(uri.path());

        let mut items: Vec<CompletionItem> = vec![
            CompletionItem {
                label: "function".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            },
            CompletionItem {
                label: "class".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            },
        ];

        // Try AI-powered completions from Claude (highest priority)
        if let Some(claude_client) = &self.claude_client {
            if let Some(content) = self.documents.get(uri.as_str()) {
                let byte_offset = position_to_byte(&content, position);
                let prefix = content[..byte_offset].to_string();
                let suffix = if byte_offset < content.len() {
                    Some(content[byte_offset..].to_string())
                } else {
                    None
                };

                let completion_context = CompletionContext {
                    language: lang.to_string(),
                    file_path: uri.path().to_string(),
                    prefix,
                    suffix,
                    context: None,
                };

                match claude_client.get_completions(&completion_context).await {
                    Ok(suggestions) => {
                        for suggestion in suggestions {
                            items.push(CompletionItem {
                                label: suggestion.text.clone(),
                                kind: Some(CompletionItemKind::TEXT),
                                detail: suggestion.detail.or(Some("Claude AI".to_string())),
                                insert_text: Some(suggestion.text),
                                sort_text: Some(format!("0_claude_{}", suggestion.confidence)),
                                ..Default::default()
                            });
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Claude completion failed: {}", e);
                    }
                }
            }
        }

        // Try AI-powered completions from GitHub Copilot
        if let Some(copilot_client) = &self.copilot_client {
            if let Some(content) = self.documents.get(uri.as_str()) {
                let byte_offset = position_to_byte(&content, position);
                let prefix = content[..byte_offset].to_string();
                let suffix = if byte_offset < content.len() {
                    Some(content[byte_offset..].to_string())
                } else {
                    None
                };

                let completion_context = CompletionContext {
                    language: lang.to_string(),
                    file_path: uri.path().to_string(),
                    prefix,
                    suffix,
                    context: None,
                };

                match copilot_client.get_completions(&completion_context).await {
                    Ok(suggestions) => {
                        for suggestion in suggestions {
                            items.push(CompletionItem {
                                label: suggestion.text.clone(),
                                kind: Some(CompletionItemKind::TEXT),
                                detail: suggestion.detail.or(Some("GitHub Copilot".to_string())),
                                insert_text: Some(suggestion.text),
                                sort_text: Some(format!("0_copilot_{}", suggestion.confidence)),
                                ..Default::default()
                            });
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Copilot completion failed: {}", e);
                    }
                }
            }
        }

        // Try tree-sitter symbol-based completions
        let lang_lowercase = lang.to_lowercase();
        if let Some(content) = self.documents.get(uri.as_str()) {
            if let Ok(mut parser) = TreeSitterParser::new() {
                if parser.set_language(&lang_lowercase).is_ok() {
                    if let Ok(tree) = parser.parse(&content, uri.as_str()) {
                        if let Ok(symbols) = parser.extract_symbols(&tree, &content, &lang_lowercase) {
                            for symbol in symbols {
                                let completion_kind = match symbol.kind {
                                    SymbolKind::FUNCTION | SymbolKind::METHOD => CompletionItemKind::FUNCTION,
                                    SymbolKind::CLASS => CompletionItemKind::CLASS,
                                    SymbolKind::VARIABLE => CompletionItemKind::VARIABLE,
                                    SymbolKind::CONSTANT => CompletionItemKind::CONSTANT,
                                    _ => CompletionItemKind::TEXT,
                                };

                                items.push(CompletionItem {
                                    label: symbol.name.clone(),
                                    kind: Some(completion_kind),
                                    detail: symbol.detail.or(Some(format!("{:?}", symbol.kind))),
                                    sort_text: Some(format!("1_{}", symbol.name)),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }

        // Query MCP servers via Coordinator (if available)
        if let Some(coordinator) = &self.coordinator_client {
            // Try querying configured MCP servers for this language
            // For now, query a generic "completion" server if it exists
            let mcp_request = McpRequest {
                request_type: "completion".to_string(),
                uri: uri.to_string(),
                position: lsp_position_to_mcp(position.line, position.character),
                context: None,
            };

            // Query all configured MCP servers via coordinator
            for server_name in self.config.mcp.servers.keys() {
                match coordinator.query(server_name, mcp_request.clone()).await {
                    Ok(response) => {
                        for suggestion in response.suggestions {
                            items.push(CompletionItem {
                                label: suggestion.clone(),
                                kind: Some(CompletionItemKind::TEXT),
                                detail: Some(format!("MCP: {}", server_name)),
                                sort_text: Some(format!("2_mcp_{}", suggestion)),
                                ..Default::default()
                            });
                        }
                    }
                    Err(e) => {
                        tracing::debug!("MCP query to {} failed: {}", server_name, e);
                    }
                }
            }
        }

        // MCP pre-processing
        if let Some(pipeline) = &self.pipeline {
            if pipeline.has_pre_processing() {
                let mcp_request = McpRequest {
                    request_type: "completion".to_string(),
                    uri: uri.to_string(),
                    position: lsp_position_to_mcp(position.line, position.character),
                    context: None,
                };

                if let Ok(responses) = pipeline.pre_process(mcp_request).await {
                    if !responses.is_empty() {
                        let merged = merge_mcp_responses(responses);
                        for suggestion in merged.suggestions {
                            items.push(CompletionItem {
                                label: suggestion.clone(),
                                kind: Some(CompletionItemKind::TEXT),
                                detail: Some("AI-powered suggestion".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }

        // Try proxy if available
        if let Some(proxy_manager) = &self.proxy_manager {
            if proxy_manager.has_proxy_for(lang) {
                self.client.log_message(
                    MessageType::INFO,
                    format!("Would proxy completion request to {} server", lang)
                ).await;
            }
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let lang = detect_language(uri.path());
        let lang_lowercase = lang.to_lowercase();

        if let Some(content) = self.documents.get(uri.as_str()) {
            let Ok(mut parser) = TreeSitterParser::new() else {
                return Ok(None);
            };
            if parser.set_language(&lang_lowercase).is_ok() {
                if let Ok(tree) = parser.parse(&content, uri.as_str()) {
                    if let Ok(Some(def)) = parser.find_definition(&tree, &content, position, &lang_lowercase) {
                        return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                            uri: uri.clone(),
                            range: def.range,
                        })));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let lang = detect_language(uri.path());
        let lang_lowercase = lang.to_lowercase();

        if let Some(content) = self.documents.get(uri.as_str()) {
            let Ok(mut parser) = TreeSitterParser::new() else {
                return Ok(None);
            };
            if parser.set_language(&lang_lowercase).is_ok() {
                if let Ok(tree) = parser.parse(&content, uri.as_str()) {
                    if let Ok(refs) = parser.find_references(&tree, &content, position, &lang_lowercase) {
                        let locations: Vec<Location> = refs
                            .into_iter()
                            .map(|r| Location {
                                uri: uri.clone(),
                                range: r.range,
                            })
                            .collect();
                        return Ok(Some(locations));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        let lang = detect_language(uri.path());
        let lang_lowercase = lang.to_lowercase();

        if let Some(content) = self.documents.get(uri.as_str()) {
            let Ok(mut parser) = TreeSitterParser::new() else {
                return Ok(None);
            };
            if parser.set_language(&lang_lowercase).is_ok() {
                if let Ok(tree) = parser.parse(&content, uri.as_str()) {
                    if let Ok(symbols) = parser.extract_symbols(&tree, &content, &lang_lowercase) {
                        let doc_symbols: Vec<DocumentSymbol> = symbols
                            .into_iter()
                            .map(|s| DocumentSymbol {
                                name: s.name,
                                detail: s.detail,
                                kind: s.kind,
                                range: s.range,
                                selection_range: s.selection_range,
                                children: None,
                                tags: None,
                                deprecated: None,
                            })
                            .collect();

                        if !doc_symbols.is_empty() {
                            return Ok(Some(DocumentSymbolResponse::Nested(doc_symbols)));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        let uri = params.text_document.uri.clone();
        let content = params.text_document.text.clone();

        self.text_sync_manager.did_open(params);
        self.documents.insert(uri_str.clone(), content.clone());

        // Compute and publish initial diagnostics
        let lang = detect_language(uri.path());
        let lang_lowercase = lang.to_lowercase();

        if let Ok(mut parser) = TreeSitterParser::new() {
            if parser.set_language(&lang_lowercase).is_ok() {
                if let Ok(tree) = parser.parse(&content, uri.as_str()) {
                    let claude_client = self.claude_client.as_deref();
                    match diagnostics::compute_diagnostics(&tree, &content, &lang_lowercase, claude_client).await {
                        Ok(mut diags) => {
                            // Enhance diagnostics with MCP validation
                            if let Some(coordinator) = &self.coordinator_client {
                                let mcp_request = McpRequest {
                                    request_type: "diagnostics".to_string(),
                                    uri: uri.to_string(),
                                    position: lsp_position_to_mcp(0, 0),
                                    context: Some(content.clone()),
                                };

                                // Query all configured MCP servers for diagnostic suggestions
                                for server_name in self.config.mcp.servers.keys() {
                                    match coordinator.query(server_name, mcp_request.clone()).await {
                                        Ok(response) => {
                                            // Convert MCP suggestions to LSP diagnostics
                                            for suggestion in response.suggestions {
                                                diags.push(Diagnostic {
                                                    range: Range {
                                                        start: tower_lsp::lsp_types::Position { line: 0, character: 0 },
                                                        end: tower_lsp::lsp_types::Position { line: 0, character: 0 },
                                                    },
                                                    severity: Some(DiagnosticSeverity::HINT),
                                                    code: None,
                                                    source: Some(format!("mcp:{}", server_name)),
                                                    message: suggestion,
                                                    related_information: None,
                                                    tags: None,
                                                    code_description: None,
                                                    data: None,
                                                });
                                            }

                                            // Add documentation as info diagnostic if present
                                            if let Some(doc) = response.documentation {
                                                diags.push(Diagnostic {
                                                    range: Range {
                                                        start: tower_lsp::lsp_types::Position { line: 0, character: 0 },
                                                        end: tower_lsp::lsp_types::Position { line: 0, character: 0 },
                                                    },
                                                    severity: Some(DiagnosticSeverity::INFORMATION),
                                                    code: None,
                                                    source: Some(format!("mcp:{}", server_name)),
                                                    message: doc,
                                                    related_information: None,
                                                    tags: None,
                                                    code_description: None,
                                                    data: None,
                                                });
                                            }
                                        }
                                        Err(e) => {
                                            tracing::debug!("MCP diagnostics query to {} failed: {}", server_name, e);
                                        }
                                    }
                                }
                            }

                            self.client.publish_diagnostics(uri, diags, None).await;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to compute initial diagnostics: {}", e);
                        }
                    }
                }
            }
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri_str = params.text_document.uri.to_string();
        let uri = params.text_document.uri.clone();

        if let Err(e) = self.text_sync_manager.did_change(params) {
            tracing::error!("Failed to apply incremental changes: {}", e);
        }

        if let Some(content) = self.text_sync_manager.get_content(&uri_str) {
            self.documents.insert(uri_str.clone(), content.clone());

            // Compute and publish diagnostics in real-time
            let lang = detect_language(uri.path());
            let lang_lowercase = lang.to_lowercase();

            // Parse with tree-sitter
            if let Ok(mut parser) = TreeSitterParser::new() {
                if parser.set_language(&lang_lowercase).is_ok() {
                    if let Ok(tree) = parser.parse(&content, uri.as_str()) {
                        // Compute diagnostics
                        let claude_client = self.claude_client.as_deref();
                        match diagnostics::compute_diagnostics(&tree, &content, &lang_lowercase, claude_client).await {
                            Ok(mut diags) => {
                                // Enhance diagnostics with MCP validation
                                if let Some(coordinator) = &self.coordinator_client {
                                    let mcp_request = McpRequest {
                                        request_type: "diagnostics".to_string(),
                                        uri: uri.to_string(),
                                        position: lsp_position_to_mcp(0, 0),
                                        context: Some(content.clone()),
                                    };

                                    // Query all configured MCP servers for diagnostic suggestions
                                    for server_name in self.config.mcp.servers.keys() {
                                        match coordinator.query(server_name, mcp_request.clone()).await {
                                            Ok(response) => {
                                                // Convert MCP suggestions to LSP diagnostics
                                                for suggestion in response.suggestions {
                                                    diags.push(Diagnostic {
                                                        range: Range {
                                                            start: tower_lsp::lsp_types::Position { line: 0, character: 0 },
                                                            end: tower_lsp::lsp_types::Position { line: 0, character: 0 },
                                                        },
                                                        severity: Some(DiagnosticSeverity::HINT),
                                                        code: None,
                                                        source: Some(format!("mcp:{}", server_name)),
                                                        message: suggestion,
                                                        related_information: None,
                                                        tags: None,
                                                        code_description: None,
                                                        data: None,
                                                    });
                                                }

                                                // Add documentation as info diagnostic if present
                                                if let Some(doc) = response.documentation {
                                                    diags.push(Diagnostic {
                                                        range: Range {
                                                            start: tower_lsp::lsp_types::Position { line: 0, character: 0 },
                                                            end: tower_lsp::lsp_types::Position { line: 0, character: 0 },
                                                        },
                                                        severity: Some(DiagnosticSeverity::INFORMATION),
                                                        code: None,
                                                        source: Some(format!("mcp:{}", server_name)),
                                                        message: doc,
                                                        related_information: None,
                                                        tags: None,
                                                        code_description: None,
                                                        data: None,
                                                    });
                                                }
                                            }
                                            Err(e) => {
                                                tracing::debug!("MCP diagnostics query to {} failed: {}", server_name, e);
                                            }
                                        }
                                    }
                                }

                                // Publish diagnostics to client
                                self.client.publish_diagnostics(uri, diags, None).await;
                            }
                            Err(e) => {
                                tracing::warn!("Failed to compute diagnostics: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();

        self.text_sync_manager.did_close(params);
        self.documents.remove(&uri);
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        for added in params.event.added {
            if let Err(e) = self.workspace_manager.add_folder(added) {
                tracing::error!("Failed to add workspace folder: {}", e);
            }
        }

        for removed in params.event.removed {
            if let Err(e) = self.workspace_manager.remove_folder(&removed.uri) {
                tracing::error!("Failed to remove workspace folder: {}", e);
            }
        }

        self.client
            .log_message(
                MessageType::INFO,
                format!("Workspace folders updated. Total: {}", self.workspace_manager.count()),
            )
            .await;
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let range = params.range;
        let diagnostics = params.context.diagnostics;
        let lang = detect_language(uri.path());

        if let Some(content) = self.documents.get(uri.as_str()) {
            match self.code_action_provider.get_actions(uri, range, &content, diagnostics, lang) {
                Ok(actions) => Ok(Some(actions)),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let lang = detect_language(uri.path());

        if let Some(content) = self.documents.get(uri.as_str()) {
            match self.signature_help_provider.get_signature_help(&content, position, lang) {
                Ok(help) => Ok(help),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = &params.text_document.uri;
        let range = params.range;
        let lang = detect_language(uri.path());

        if let Some(content) = self.documents.get(uri.as_str()) {
            match self.inlay_hints_provider.get_inlay_hints(&content, range, lang) {
                Ok(hints) => Ok(Some(hints)),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;
        let lang = detect_language(uri.path());

        if let Some(content) = self.documents.get(uri.as_str()) {
            match self.formatting_provider.format_document(&content, lang, uri) {
                Ok(edits) => Ok(Some(edits)),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;
        let range = params.range;
        let lang = detect_language(uri.path());

        if let Some(content) = self.documents.get(uri.as_str()) {
            match self.formatting_provider.format_range(&content, range, lang, uri) {
                Ok(edits) => Ok(Some(edits)),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;
        let lang = detect_language(uri.path());

        if let Some(content) = self.documents.get(uri.as_str()) {
            match self.semantic_tokens_provider.get_semantic_tokens(&content, lang) {
                Ok(Some(tokens)) => Ok(Some(SemanticTokensResult::Tokens(tokens))),
                Ok(None) => Ok(None),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
}

/// Helper function to convert LSP position to byte offset
fn position_to_byte(source: &str, position: Position) -> usize {
    let mut byte_offset = 0;
    let mut current_line = 0;
    let mut current_char = 0;

    for ch in source.chars() {
        if current_line == position.line && current_char == position.character {
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

/// Start LSP server mode
async fn run_lsp_server(config: Config) {
    // Initialize logging with configured level
    let log_level = match config.server.log_level.as_str() {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        "debug" => tracing::Level::DEBUG,
        "trace" => tracing::Level::TRACE,
        _ => tracing::Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    tracing::info!("Universal LSP Server starting...");
    tracing::info!("Configuration: MCP pipeline: {}, Proxy servers: {}",
        config.has_mcp_pipeline(),
        config.has_proxy_servers()
    );

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| {
        UniversalLsp::new(client, config.clone())
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Start ACP agent mode (placeholder)
async fn run_acp_agent(config: Config) {
    use universal_lsp::acp;

    println!("🤖 Universal LSP ACP Agent starting...");
    println!("📋 Configuration:");
    println!("   • MCP servers configured: {}", config.mcp.servers.len());
    println!("   • Log level: {}", config.server.log_level);
    println!("   • Max concurrent requests: {}", config.server.max_concurrent);

    if config.mcp.servers.is_empty() {
        println!("\n⚠️  No MCP servers configured - running in standalone mode");
        println!("   Add --mcp-server flags to enable MCP integration");
    } else {
        println!("\n✅ MCP integration enabled:");
        for (name, server_config) in &config.mcp.servers {
            println!("   • {}: {}", name, server_config.target);
        }
    }

    println!("\n🚀 Starting ACP agent on stdio...\n");

    // Run the ACP agent
    if let Err(e) = acp::run_agent().await {
        eprintln!("❌ ACP agent error: {}", e);
        std::process::exit(1);
    }
}

/// Initialize Zed workspace with comprehensive MCP configuration
async fn run_zed_init(
    path: std::path::PathBuf,
    name: Option<String>,
    with_mcp: bool,
    with_claude: bool,
    with_copilot: bool,
    with_acp: bool,
) {
    use std::fs;
    use serde_json::{json, Value};

    println!("🚀 Universal LSP - Zed Workspace Initialization");
    println!("===============================================\n");
    println!("📂 Project path: {}", path.display());

    let project_name = name.unwrap_or_else(|| {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string()
    });
    println!("📝 Project name: {}", project_name);

    // Create .zed directory if it doesn't exist
    let zed_dir = path.join(".zed");
    if let Err(e) = fs::create_dir_all(&zed_dir) {
        eprintln!("❌ Failed to create .zed directory: {}", e);
        std::process::exit(1);
    }

    // Build comprehensive settings
    let mut settings = json!({
        "language_servers": {
            "universal-lsp": {
                "command": "universal-lsp",
                "args": ["lsp"]
            }
        },
        "lsp": {
            "universal-lsp": {
                "initialization_options": {
                    "enable_hover": true,
                    "enable_completion": true,
                    "enable_diagnostics": true
                }
            },
            "vtsls": {
                "initialization_options": {
                    "enabled": false
                }
            },
            "eslint": {
                "initialization_options": {
                    "enabled": false
                }
            },
            "rust-analyzer": {
                "initialization_options": {
                    "enabled": false
                }
            },
            "pyright": {
                "initialization_options": {
                    "enabled": false
                }
            },
            "pylsp": {
                "initialization_options": {
                    "enabled": false
                }
            }
        },
        "languages": {
            // Web & JavaScript Ecosystem
            "JavaScript": {
                "language_servers": ["universal-lsp"]
            },
            "TypeScript": {
                "language_servers": ["universal-lsp"]
            },
            "TSX": {
                "language_servers": ["universal-lsp"]
            },
            // Web Core Languages
            "HTML": {
                "language_servers": ["universal-lsp"]
            },
            "CSS": {
                "language_servers": ["universal-lsp"]
            },
            "JSON": {
                "language_servers": ["universal-lsp"]
            },
            "Svelte": {
                "language_servers": ["universal-lsp"]
            },
            // Systems Languages
            "Python": {
                "language_servers": ["universal-lsp"]
            },
            "Rust": {
                "language_servers": ["universal-lsp"]
            },
            "Go": {
                "language_servers": ["universal-lsp"]
            },
            "Java": {
                "language_servers": ["universal-lsp"]
            },
            "C": {
                "language_servers": ["universal-lsp"]
            },
            "C++": {
                "language_servers": ["universal-lsp"]
            },
            // Shell
            "Bash": {
                "language_servers": ["universal-lsp"]
            },
            "Shell Script": {
                "language_servers": ["universal-lsp"]
            },
            // JVM Languages
            "Scala": {
                "language_servers": ["universal-lsp"]
            },
            "Kotlin": {
                "language_servers": ["universal-lsp"]
            },
            // .NET
            "C#": {
                "language_servers": ["universal-lsp"]
            }
        }
    });

    // Add comprehensive MCP server configuration if requested
    if with_mcp {
        println!("\n📦 Configuring MCP servers...\n");

        let mcp_servers = json!({
            // === Core Development Servers ===
            "rust-mcp-filesystem": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-filesystem", "."],
                "description": "File system operations and content reading"
            },
            "FileScopeMCP": {
                "command": "npx",
                "args": ["-y", "@joshuarileydev/filescope-mcp"],
                "description": "Advanced file scope analysis and search"
            },
            "In-Memoria": {
                "command": "npx",
                "args": ["-y", "@pi22by7/in-memoria"],
                "description": "Persistent memory and context storage"
            },
            "git-mcp": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-git"],
                "description": "Git operations and repository management"
            },
            "github-mcp-server": {
                "command": "npx",
                "args": ["-y", "@github/github-mcp-server"],
                "description": "GitHub API integration for issues, PRs, and repositories"
            },

            // === Web & API Servers ===
            "duckduckgo-mcp-server": {
                "command": "npx",
                "args": ["-y", "@nickclyde/duckduckgo-mcp-server"],
                "description": "DuckDuckGo web search integration"
            },
            "fetch-mcp": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-fetch"],
                "description": "HTTP requests and API testing"
            },
            "brave-search-mcp": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-brave-search"],
                "description": "Brave Search API integration"
            },

            // === Automation & Browser ===
            "playwright-mcp": {
                "command": "npx",
                "args": ["-y", "@microsoft/playwright-mcp"],
                "description": "Browser automation and web scraping with Playwright"
            },

            // === Database Servers ===
            "sqlite-mcp": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-sqlite"],
                "description": "SQLite database operations"
            },
            "postgres-mcp": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-postgres"],
                "description": "PostgreSQL database operations"
            },

            // === AI Enhancement ===
            "sequential-thinking-mcp": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-sequential-thinking"],
                "description": "Enhanced reasoning and step-by-step problem solving"
            },
            "memory-mcp": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-memory"],
                "description": "Persistent context and knowledge across sessions"
            },

            // === Creative & Media ===
            "everart-mcp": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-everart"],
                "description": "AI image generation for documentation and assets"
            },

            // === Infrastructure ===
            "cloudflare-mcp": {
                "command": "npx",
                "args": ["-y", "@modelcontextprotocol/server-cloudflare"],
                "description": "Cloudflare API operations and management"
            }
        });

        if let Value::Object(ref mut map) = settings {
            map.insert("mcp_servers".to_string(), mcp_servers);
        }

        println!("✅ Core Development (5 servers):");
        println!("   • rust-mcp-filesystem - File operations");
        println!("   • FileScopeMCP - Advanced file analysis");
        println!("   • In-Memoria - Persistent memory");
        println!("   • git-mcp - Git operations");
        println!("   • github-mcp-server - GitHub API");

        println!("\n✅ Web & API (3 servers):");
        println!("   • duckduckgo-mcp-server - Web search");
        println!("   • fetch-mcp - HTTP/API testing");
        println!("   • brave-search-mcp - Brave Search");

        println!("\n✅ Automation (1 server):");
        println!("   • playwright-mcp - Browser automation");

        println!("\n✅ Database (2 servers):");
        println!("   • sqlite-mcp - SQLite operations");
        println!("   • postgres-mcp - PostgreSQL operations");

        println!("\n✅ AI Enhancement (2 servers):");
        println!("   • sequential-thinking-mcp - Enhanced reasoning");
        println!("   • memory-mcp - Persistent context");

        println!("\n✅ Creative & Media (1 server):");
        println!("   • everart-mcp - AI image generation");

        println!("\n✅ Infrastructure (1 server):");
        println!("   • cloudflare-mcp - Cloudflare API");

        println!("\n📊 Total: 15 MCP servers configured");
    }

    // Add Claude AI configuration if requested
    if with_claude {
        println!("\n🤖 Configuring Claude AI integration...\n");

        let claude_config = json!({
            "provider": "anthropic",
            "model": "claude-sonnet-4-20250514",
            "api_key_from_env": "ANTHROPIC_API_KEY",
            "features": {
                "completions": true,
                "inline_assist": true,
                "chat": true
            },
            "settings": {
                "max_tokens": 4096,
                "temperature": 0.7,
                "timeout_ms": 30000
            }
        });

        if let Value::Object(ref mut map) = settings {
            map.insert("claude".to_string(), claude_config);
        }

        println!("✅ Claude AI configured:");
        println!("   • Model: claude-sonnet-4-20250514");
        println!("   • Features: Completions, Inline Assist, Chat");
        println!("   • Set ANTHROPIC_API_KEY environment variable");
    }

    // Add GitHub Copilot configuration if requested
    if with_copilot {
        println!("\n🐙 Configuring GitHub Copilot integration...\n");

        let copilot_config = json!({
            "provider": "github",
            "features": {
                "completions": true,
                "inline_assist": true
            },
            "settings": {
                "enable_auto_completions": true,
                "debounce_ms": 75
            }
        });

        if let Value::Object(ref mut map) = settings {
            map.insert("copilot".to_string(), copilot_config);
        }

        println!("✅ GitHub Copilot configured:");
        println!("   • Auto-completions enabled");
        println!("   • Inline suggestions enabled");
        println!("   • Sign in to GitHub when prompted");
    }

    // Add ACP agent configuration if requested
    if with_acp {
        println!("\n🔮 Configuring ACP (Agent Client Protocol) integration...\n");

        let acp_config = json!({
            "enabled": true,
            "agent_servers": {
                "local": {
                    "command": "universal-lsp",
                    "args": ["acp"],
                    "description": "Local Universal LSP ACP agent"
                }
            },
            "features": {
                "agent_assist": true,
                "multi_turn_conversations": true,
                "context_awareness": true
            },
            "settings": {
                "max_conversation_turns": 10,
                "context_window": 8192
            }
        });

        if let Value::Object(ref mut map) = settings {
            map.insert("acp".to_string(), acp_config);
        }

        println!("✅ ACP Agent configured:");
        println!("   • Local agent server enabled");
        println!("   • Multi-turn conversations supported");
        println!("   • Context-aware assistance enabled");
    }

    // Display language support information
    println!("\n🔧 Configured Universal LSP for 18 languages:");
    println!("   • JavaScript, TypeScript, TSX");
    println!("   • HTML, CSS, JSON, Svelte");
    println!("   • Python, Rust, Go, Java, C, C++");
    println!("   • Bash, Shell, Scala, Kotlin, C#");

    // Write settings.json
    let settings_path = zed_dir.join("settings.json");
    let settings_json = serde_json::to_string_pretty(&settings)
        .expect("Failed to serialize settings");

    if let Err(e) = fs::write(&settings_path, settings_json) {
        eprintln!("❌ Failed to write settings.json: {}", e);
        std::process::exit(1);
    }

    println!("\n✨ Workspace initialized successfully!");
    println!("📄 Configuration written to: {}", settings_path.display());

    // Summary of configured features
    let mut feature_count = 1; // Universal LSP is always configured
    if with_mcp { feature_count += 1; }
    if with_claude { feature_count += 1; }
    if with_copilot { feature_count += 1; }
    if with_acp { feature_count += 1; }

    println!("\n📊 Configuration Summary:");
    println!("   ✓ Universal LSP (18 languages)");
    if with_mcp {
        println!("   ✓ MCP servers (15 servers configured)");
    }
    if with_claude {
        println!("   ✓ Claude AI (Sonnet 4)");
    }
    if with_copilot {
        println!("   ✓ GitHub Copilot");
    }
    if with_acp {
        println!("   ✓ ACP Agent");
    }

    println!("\n💡 Next steps:");
    println!("   1. Open this directory in Zed editor");

    if with_mcp {
        println!("   2. MCP servers will auto-start when needed");
        println!("   3. Universal LSP provides completions and hover info");

        if with_claude {
            println!("   4. Set ANTHROPIC_API_KEY environment variable for Claude AI");
            println!("   5. Use Ctrl+Space for AI-powered completions");
        } else if with_copilot {
            println!("   4. Sign in to GitHub for Copilot access");
            println!("   5. Use Ctrl+Space for AI-powered completions");
        } else {
            println!("   4. Language support enabled for 18 languages");
            println!("   5. Use Ctrl+Space for completions");
        }

        println!("\n📚 MCP servers require Node.js/npm to be installed");
        println!("   They will be auto-installed via npx when first used");
    } else {
        if with_claude || with_copilot || with_acp {
            println!("   2. Universal LSP enabled for 18 languages");

            if with_claude {
                println!("   3. Set ANTHROPIC_API_KEY environment variable for Claude AI");
                println!("   4. Use Ctrl+Space for AI-powered completions");
            } else if with_copilot {
                println!("   3. Sign in to GitHub for Copilot access");
                println!("   4. Use Ctrl+Space for AI-powered completions");
            } else {
                println!("   3. Use Ctrl+Space for code completions");
            }
        } else {
            println!("   2. Universal LSP enabled for 18 languages");
            println!("   3. Use Ctrl+Space for code completions");
        }

        println!("\n💡 To add more features, run with flags:");
        println!("   ulsp zed init --with-mcp          # Add MCP servers");
        println!("   ulsp zed init --with-claude       # Add Claude AI");
        println!("   ulsp zed init --with-copilot      # Add GitHub Copilot");
        println!("   ulsp zed init --with-acp          # Add ACP agent");
        println!("   ulsp zed init --with-mcp --with-claude --with-copilot --with-acp  # All features");
    }

    std::process::exit(0);
}

#[tokio::main]
async fn main() {
    // Parse configuration and command mode
    let (config, mode) = Config::from_args().expect("Failed to load configuration");

    // Route to appropriate handler based on command mode
    match mode {
        CommandMode::Lsp => run_lsp_server(config).await,
        CommandMode::Acp => run_acp_agent(config).await,
        CommandMode::ZedInit { path, name, with_mcp, with_claude, with_copilot, with_acp } => {
            run_zed_init(path, name, with_mcp, with_claude, with_copilot, with_acp).await
        }
    }
}
