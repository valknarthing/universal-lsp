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

mod config;
mod language;
mod mcp;
mod pipeline;
mod proxy;
mod tree_sitter;

use config::Config;
use language::detect_language;
use pipeline::{McpPipeline, merge_mcp_responses, lsp_position_to_mcp};
use proxy::{ProxyConfig, ProxyManager};
use mcp::McpRequest;
use tree_sitter::TreeSitterParser;

#[derive(Debug)]
struct UniversalLsp {
    client: Client,
    config: Arc<Config>,
    pipeline: Option<Arc<McpPipeline>>,
    proxy_manager: Option<Arc<ProxyManager>>,
    parser: Arc<dashmap::DashMap<String, TreeSitterParser>>,
    documents: Arc<dashmap::DashMap<String, String>>,
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

        Self {
            client,
            config: Arc::new(config),
            pipeline,
            proxy_manager,
            parser: Arc::new(dashmap::DashMap::new()),
            documents: Arc::new(dashmap::DashMap::new()),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for UniversalLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
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

        // Try MCP pre-processing if available
        let mut hover_text = format!("Language: {}\n\nUniversal LSP Server", lang);

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

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let content = params.text_document.text;
        self.documents.insert(uri, content);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.insert(uri, change.text);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.documents.remove(&uri);
    }
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
