//! Universal LSP Server
//! Supports 242+ programming languages with AI-powered intelligent code analysis
//!
//! Features:
//! - MCP pipeline for AI-powered pre/post-processing
//! - LSP proxy to specialized language servers
//! - CLI-based configuration

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
mod text_sync;
mod tree_sitter;
mod workspace;

use ai::{ClaudeClient, ClaudeConfig, CopilotClient, CopilotConfig, CompletionContext};
use code_actions::CodeActionProvider;
use config::Config;
use diagnostics::DiagnosticProvider;
use formatting::FormattingProvider;
use language::detect_language;
use mcp::McpRequest;
use pipeline::{McpPipeline, merge_mcp_responses, lsp_position_to_mcp};
use proxy::{ProxyConfig, ProxyManager};
use text_sync::TextSyncManager;
use tree_sitter::TreeSitterParser;
use workspace::WorkspaceManager;

struct UniversalLsp {
    client: Client,
    config: Arc<Config>,
    pipeline: Option<Arc<McpPipeline>>,
    proxy_manager: Option<Arc<ProxyManager>>,
    claude_client: Option<Arc<ClaudeClient>>,
    copilot_client: Option<Arc<CopilotClient>>,
    parser: Arc<dashmap::DashMap<String, TreeSitterParser>>,
    documents: Arc<dashmap::DashMap<String, String>>,
    diagnostic_provider: Arc<DiagnosticProvider>,
    code_action_provider: Arc<CodeActionProvider>,
    formatting_provider: Arc<FormattingProvider>,
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

        Self {
            client,
            config: Arc::new(config),
            pipeline,
            proxy_manager,
            claude_client,
            copilot_client,
            parser: Arc::new(dashmap::DashMap::new()),
            documents: Arc::new(dashmap::DashMap::new()),
            diagnostic_provider: Arc::new(DiagnosticProvider::new()),
            code_action_provider: Arc::new(CodeActionProvider::new()),
            formatting_provider: Arc::new(FormattingProvider::new()),
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
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
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

        let mut hover_text = format!("Language: {}", lang);

        // Try tree-sitter symbol extraction at cursor position
        if let Some(content) = self.documents.get(uri.as_str()) {
            if let Ok(mut parser) = TreeSitterParser::new() {
                if parser.set_language(lang).is_ok() {
                    if let Ok(tree) = parser.parse(&content, uri.as_str()) {
                        // Find node at position
                        let byte_offset = position_to_byte(&content, position);
                        if let Some(node) = tree.root_node().descendant_for_byte_range(byte_offset, byte_offset) {
                            // Get symbol info
                            let node_text = &content[node.byte_range()];
                            let kind = node.kind();

                            hover_text = format!(
                                "{}\n\nSymbol: {}\nType: {}\nPosition: {}:{}",
                                hover_text, node_text, kind,
                                position.line, position.character
                            );
                        }
                    }
                }
            }
        }

        // Try MCP pre-processing if available

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
                            hover_text = format!("{}\n\n{}", hover_text, doc);
                        }
                    }
                }
            }
        }

        // Try proxy if available
        if let Some(proxy_manager) = &self.proxy_manager {
            if proxy_manager.has_proxy_for(lang) {
                // TODO: Forward to proxy and merge results
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
                // Extract prefix (code before cursor) and suffix (code after cursor)
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
                    context: None, // Could add surrounding context in future
                };

                match claude_client.get_completions(&completion_context).await {
                    Ok(suggestions) => {
                        for suggestion in suggestions {
                            items.push(CompletionItem {
                                label: suggestion.text.clone(),
                                kind: Some(CompletionItemKind::TEXT),
                                detail: suggestion.detail.or(Some("Claude AI".to_string())),
                                insert_text: Some(suggestion.text),
                                sort_text: Some(format!("0_claude_{}", suggestion.confidence)), // Highest priority
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
                // Extract prefix (code before cursor) and suffix (code after cursor)
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
                                sort_text: Some(format!("0_copilot_{}", suggestion.confidence)), // Highest priority
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
        if let Some(content) = self.documents.get(uri.as_str()) {
            if let Ok(mut parser) = TreeSitterParser::new() {
                if parser.set_language(lang).is_ok() {
                    if let Ok(tree) = parser.parse(&content, uri.as_str()) {
                        if let Ok(symbols) = parser.extract_symbols(&tree, &content, lang) {
                            // Add symbols as completion items
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
                                    sort_text: Some(format!("1_{}", symbol.name)), // Lower priority than AI
                                    ..Default::default()
                                });
                            }
                        }
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
                        // Add MCP suggestions as completion items
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
                // TODO: Forward to proxy and merge results
            }
        }

        // MCP post-processing
        if let Some(pipeline) = &self.pipeline {
            if pipeline.has_post_processing() {
                let mcp_request = McpRequest {
                    request_type: "completion".to_string(),
                    uri: uri.to_string(),
                    position: lsp_position_to_mcp(position.line, position.character),
                    context: Some(format!("Items: {:?}", items.len())),
                };

                if let Ok(responses) = pipeline.post_process(mcp_request, "").await {
                    if !responses.is_empty() {
                        let merged = merge_mcp_responses(responses);
                        // Add post-processed suggestions
                        for suggestion in merged.suggestions {
                            items.push(CompletionItem {
                                label: suggestion.clone(),
                                kind: Some(CompletionItemKind::TEXT),
                                detail: Some("AI-enhanced".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                }
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

        // Try to get document content
        if let Some(content) = self.documents.get(uri.as_str()) {
            // Try tree-sitter based definition finding
            let Ok(mut parser) = TreeSitterParser::new() else {
                return Ok(None);
            };
            if parser.set_language(lang).is_ok() {
                if let Ok(tree) = parser.parse(&content, uri.as_str()) {
                    if let Ok(Some(def)) = parser.find_definition(&tree, &content, position, lang) {
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

        // Try to get document content
        if let Some(content) = self.documents.get(uri.as_str()) {
            // Try tree-sitter based reference finding
            let Ok(mut parser) = TreeSitterParser::new() else {
                return Ok(None);
            };
            if parser.set_language(lang).is_ok() {
                if let Ok(tree) = parser.parse(&content, uri.as_str()) {
                    if let Ok(refs) = parser.find_references(&tree, &content, position, lang) {
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

        // Try to get document content
        if let Some(content) = self.documents.get(uri.as_str()) {
            // Try tree-sitter based symbol extraction
            let Ok(mut parser) = TreeSitterParser::new() else {
                return Ok(None);
            };
            if parser.set_language(lang).is_ok() {
                if let Ok(tree) = parser.parse(&content, uri.as_str()) {
                    if let Ok(symbols) = parser.extract_symbols(&tree, &content, lang) {
                        // Convert to LSP DocumentSymbol format
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
        let uri = params.text_document.uri.to_string();
        let content = params.text_document.text.clone();

        // Use text_sync_manager for incremental tracking
        self.text_sync_manager.did_open(params);

        // Keep old documents map for compatibility
        self.documents.insert(uri, content);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();

        // Use text_sync_manager for incremental changes
        if let Err(e) = self.text_sync_manager.did_change(params) {
            tracing::error!("Failed to apply incremental changes: {}", e);
        }

        // Update old documents map for compatibility
        if let Some(content) = self.text_sync_manager.get_content(&uri) {
            self.documents.insert(uri, content);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();

        // Close in both managers
        self.text_sync_manager.did_close(params);
        self.documents.remove(&uri);
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        // Add new folders
        for added in params.event.added {
            if let Err(e) = self.workspace_manager.add_folder(added) {
                tracing::error!("Failed to add workspace folder: {}", e);
            }
        }

        // Remove folders
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

#[tokio::main]
async fn main() {
    // Parse configuration from command-line arguments
    let config = Config::from_args().expect("Failed to load configuration");

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
