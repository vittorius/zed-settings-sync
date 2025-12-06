use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;
use serde_json::from_value;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
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
    // Mutex is needed for interior mutability, Option - for delayed initialization;
    // both of them - because of how the LanguageServer trait is defined by tower-lsp:
    // its methods don't mutate self.
    // We have to use the async Mutex here because the `std::sync::Mutex` is not Send
    // thus cannot be used in async functions.
    app_state: Mutex<Option<AppState>>,
}

impl Backend {
    pub fn new(_client: Client) -> Self {
        Self {
            app_state: Mutex::new(None),
        }
    }

    async fn watch_path(&self, path: PathBuf) -> Result<()> {
        let info_msg = format!("Watching path: {}", path.display());

        #[allow(clippy::expect_used)]
        self.app_state
            .lock()
            .await
            .as_ref()
            .expect("App state must be already initialized")
            .watcher_store
            .watch(path)
            .await?;

        info!("{}", info_msg);

        Ok(())
    }

    async fn unwatch_path(&self, path: PathBuf) -> Result<()> {
        let info_msg = format!("Unwatching path: {}", path.display());

        #[allow(clippy::expect_used)]
        self.app_state
            .lock()
            .await
            .as_ref()
            .expect("App state must be already initialized")
            .watcher_store
            .unwatch(path)
            .await?;

        info!("{}", info_msg);

        Ok(())
    }
}

// TODO: extract into a shared crate for use by both LSP and CLI tool
#[derive(Debug, Deserialize)]
struct Config {
    gist_id: String,
    github_token: String,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        info!("Initializing Zed Settings Sync LSP...");

        let options = params.initialization_options.ok_or_else(|| {
            error!("initialization_options are missing from LSP server configuration");
            tower_lsp::jsonrpc::Error::internal_error()
        })?;
        let config: Config = from_value(options).map_err(|err| {
            error!("Failed to deserialize initialization_options: {}", err);
            tower_lsp::jsonrpc::Error::internal_error()
        })?;

        let app_state = AppState::new(config.gist_id, config.github_token).map_err(|err| {
            error!("Failed to build the app state: {}", err);
            tower_lsp::jsonrpc::Error::internal_error()
        })?;

        {
            let mut shared_app_state = self.app_state.lock().await;

            shared_app_state.replace(app_state);

            #[allow(clippy::expect_used)]
            shared_app_state
                .as_ref()
                .expect("App state should have been already initialized")
                .watcher_store
                .start_watcher()
                .await;
        }

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
    async fn shutdown(&self) -> LspResult<()> {
        info!("Shutting down Zed Settings Sync LSP");

        Ok(())
    }

    #[instrument(skip(self, params))]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        debug!("Document opened: {}", params.text_document.uri);

        match ZedConfigFilePath::from_file_uri(&params.text_document.uri) {
            Ok(path) => {
                let path_to_watch = path.to_watched_path_buf();
                // TODO: expose sync_client in app state and sync file explicitly after opening (quick'n'dirty way to fight losing last changes on LSP restart on settings update)
                if let Err(err) = self.watch_path(path_to_watch).await {
                    error!("Failed to start watching path: {}", err);
                }
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
                let path_to_watch = path.to_watched_path_buf();
                if let Err(err) = self.unwatch_path(path_to_watch).await {
                    error!("Failed to stop watching path: {}", err);
                }
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
