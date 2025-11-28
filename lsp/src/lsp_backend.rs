use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{DidCloseTextDocumentParams, DidOpenTextDocumentParams};
use tower_lsp::{
    Client, LanguageServer,
    lsp_types::{
        InitializeParams, InitializeResult, InitializedParams, ServerCapabilities, ServerInfo,
        TextDocumentSyncCapability, TextDocumentSyncOptions, WorkspaceServerCapabilities,
    },
};
use tracing::{debug, error, info, instrument};

use crate::app_state::AppState;
use crate::watching::{ZedConfigFilePath, ZedConfigPathError};

const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub struct Backend {
    app_state: AppState,
}

impl Backend {
    pub fn new(_client: Client) -> Self {
        Self {
            app_state: AppState::new().expect("Failed to create AppState"),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        info!("Initializing Zed Settings Sync LSP...");

        // start filesystem path watcher
        self.app_state.watcher_store.start_watcher().await;

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: CARGO_PKG_NAME.into(),
                version: Some(CARGO_PKG_VERSION.into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        ..TextDocumentSyncOptions::default()
                    },
                )),
                workspace: Some(WorkspaceServerCapabilities {
                    file_operations: None,
                    workspace_folders: None,
                }),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("Zed Settings Sync LSP server fully initialized and ready");
    }

    #[instrument(skip(self))]
    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Zed Settings Sync LSP");

        Ok(())
    }

    #[instrument(skip(self, params))]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        debug!("Document opened: {}", params.text_document.uri);

        match ZedConfigFilePath::from_file_uri(&params.text_document.uri) {
            Ok(path) => {
                let path_to_watch = path.to_watched_path_buf();
                // TODO: handle error
                info!("Watching path: {}", path_to_watch.display());
                let _ = self.app_state.watcher_store.watch(path_to_watch).await;
            }
            Err(ZedConfigPathError::NotZedConfigFile) => {
                debug!(
                    "Not a Zed config file, skipping: {}",
                    params.text_document.uri
                );
            }
            Err(ZedConfigPathError::WrongFileUriFormat) => {
                error!("Wrong file uri format: {}", params.text_document.uri);
            }
        }
    }

    #[instrument(skip(self, params))]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        debug!("Document closed: {}", params.text_document.uri);

        match ZedConfigFilePath::from_file_uri(&params.text_document.uri) {
            Ok(path) => {
                info!("Unwatching path: {}", path);
                let _ = self
                    .app_state
                    .watcher_store
                    .unwatch(path.to_watched_path_buf())
                    .await;
            }
            Err(ZedConfigPathError::NotZedConfigFile) => {
                debug!(
                    "Not a Zed config file, skipping: {}",
                    params.text_document.uri
                );
            }
            Err(ZedConfigPathError::WrongFileUriFormat) => {
                error!("Wrong file uri format: {}", params.text_document.uri);
            }
        }
    }
}
