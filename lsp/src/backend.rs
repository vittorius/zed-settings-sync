use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::Result;
use common::config::Config;
#[cfg(not(test))]
use tower_lsp::Client as LspClient;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{DidCloseTextDocumentParams, DidOpenTextDocumentParams};
use tower_lsp::{
    LanguageServer,
    lsp_types::{
        InitializeParams, InitializeResult, InitializedParams, ServerCapabilities, ServerInfo,
        TextDocumentSyncCapability, TextDocumentSyncOptions, WorkspaceServerCapabilities,
    },
};
use tracing::{debug, error, info, instrument};
use zed_extension_api::serde_json::from_value;

use crate::app_state::AppState;
#[cfg(test)]
use crate::mocks::MockLspClient as LspClient;
use crate::watching::{ZedConfigFilePath, ZedConfigPathError};

const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub struct Backend {
    // OnceLock is needed for cross-thread sync (Tokio) and for delayed initialization.
    // Mutex is needed for interior mutability over a shared reference
    // because LanguageServer trait methods accept &self (not &mut self).
    app_state: OnceLock<Mutex<AppState>>,
    lsp_client: LspClient,
}

impl Backend {
    pub fn new(lsp_client: LspClient) -> Self {
        Self {
            app_state: OnceLock::new(),
            lsp_client,
        }
    }

    fn watch_path(&self, path: PathBuf) -> Result<()> {
        let info_msg = format!("Watching path: {}", path.display());

        #[allow(clippy::expect_used)]
        self.app_state
            .get()
            .expect("App state must be initialized")
            .lock()
            .expect("Watched paths store mutex is poisoned")
            .watched_paths
            .watch(path)?;

        info!("{}", info_msg);

        Ok(())
    }

    fn unwatch_path(&self, path: &Path) -> Result<()> {
        let info_msg = format!("Unwatching path: {}", path.display());

        #[allow(clippy::expect_used)]
        self.app_state
            .get()
            .expect("App state must be initialized")
            .lock()
            .expect("Watched paths store mutex is poisoned")
            .watched_paths
            .unwatch(path)?;

        info!("{}", info_msg);

        Ok(())
    }
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

        let app_state = AppState::new(
            config.gist_id().into(),
            config.github_token().into(),
            Arc::new(self.lsp_client.clone()),
        )
        .map_err(|err| {
            error!("Failed to build the app state: {}", err);
            tower_lsp::jsonrpc::Error::internal_error()
        })?;

        #[allow(clippy::expect_used)]
        self.app_state
            .set(Mutex::new(app_state))
            .expect("AppState was already initialized");

        #[allow(clippy::expect_used)]
        self.app_state
            .get()
            .expect("App state should be initialized")
            .lock()
            .expect("Watched paths store mutex is poisoned")
            .watched_paths
            .start_watcher();

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
                // TODO: expose sync_client in app state and sync file explicitly after opening
                // (quick'n'dirty way to fight losing last changes on LSP restart on settings update)
                if let Err(err) = self.watch_path(path_to_watch) {
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
                if let Err(err) = self.unwatch_path(&path_to_watch) {
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::Path;
    use std::path::PathBuf;

    use anyhow::Result;
    use anyhow::anyhow;
    use mockall::{Sequence, predicate};
    use tower_lsp::{LanguageServer, lsp_types::InitializeParams};
    use zed_extension_api::serde_json::{Value, json};

    use crate::{backend::Backend, mocks::MockLspClient, watching::MockPathStore};

    async fn init_lsp_backend(initialization_options: Option<Value>) -> Result<Backend> {
        let mut mock_lsp_client = MockLspClient::default();
        mock_lsp_client
            .expect_clone()
            .returning(MockLspClient::default);

        let backend = Backend::new(mock_lsp_client);
        let initialize_params = InitializeParams {
            initialization_options,
            ..Default::default()
        };
        backend.initialize(initialize_params).await?;

        Ok(backend)
    }

    async fn init_lsp_backend_default() -> Result<Backend> {
        init_lsp_backend(Some(json!({
            "github_token": "gho_my-shiny-token",
            "gist_id": "deadbeefdeadbeefdeadbeefdeadbeef"
        })))
        .await
    }

    #[tokio::test]
    async fn test_initialize_success() -> Result<()> {
        let ctx = MockPathStore::new_context();
        ctx.expect().returning(|_, _| {
            let mut mock_path_store = MockPathStore::default();
            mock_path_store.expect_start_watcher().returning(|| ());
            Ok(mock_path_store)
        });

        init_lsp_backend_default().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_failure_missing_initialization_options() -> Result<()> {
        let ctx = MockPathStore::new_context();
        ctx.expect().returning(|_, _| {
            let mut mock_path_store = MockPathStore::default();
            mock_path_store.expect_start_watcher().returning(|| ());
            Ok(mock_path_store)
        });

        assert!(init_lsp_backend(None).await.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_initialize_failure_invalid_initialization_options() -> Result<()> {
        let ctx = MockPathStore::new_context();
        ctx.expect().returning(|_, _| {
            let mut mock_path_store = MockPathStore::default();
            mock_path_store.expect_start_watcher().returning(|| ());
            Ok(mock_path_store)
        });

        let test_cases = [
            json!({
               "hello": "world"
            }),
            json!({
               "gist_id": "1234"
            }),
            json!({
               "github_token": "tok"
            }),
            json!({
               "gist_id": "1234",
               "random_key": "value"
            }),
            json!({
               "github_token": "5678",
               "random_prop": "val"
            }),
        ];

        for test in test_cases {
            assert!(init_lsp_backend(Some(test)).await.is_err());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_watch_path_success() -> Result<()> {
        let path = "/path/to/watch";

        let ctx = MockPathStore::new_context();
        ctx.expect().returning(|_, _| {
            let mut seq = Sequence::new();
            let mut mock_path_store = MockPathStore::default();
            mock_path_store
                .expect_start_watcher()
                .in_sequence(&mut seq)
                .returning(|| ());
            mock_path_store
                .expect_watch()
                .in_sequence(&mut seq)
                .with(predicate::eq(PathBuf::from(path.to_string())))
                .returning(|_| Ok(()));
            Ok(mock_path_store)
        });

        let backend = init_lsp_backend_default().await?;

        backend.watch_path(PathBuf::from(path))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_watch_path_failure_path_store_watch_failed() -> Result<()> {
        let path = "/path/to/watch";

        let ctx = MockPathStore::new_context();
        ctx.expect().returning(|_, _| {
            let mut seq = Sequence::new();
            let mut mock_path_store = MockPathStore::default();
            mock_path_store
                .expect_start_watcher()
                .in_sequence(&mut seq)
                .returning(|| ());
            mock_path_store
                .expect_watch()
                .in_sequence(&mut seq)
                .with(predicate::eq(PathBuf::from(path.to_string())))
                .returning(|_| Err(anyhow!("Failed to watch path")));
            Ok(mock_path_store)
        });

        let backend = init_lsp_backend_default().await?;

        assert_eq!(
            backend
                .watch_path(PathBuf::from(path))
                .unwrap_err()
                .to_string(),
            "Failed to watch path"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_unwatch_path_success() -> Result<()> {
        let path = "/path/to/watch";

        let ctx = MockPathStore::new_context();
        ctx.expect().returning(|_, _| {
            let mut seq = Sequence::new();
            let mut mock_path_store = MockPathStore::default();
            mock_path_store
                .expect_start_watcher()
                .in_sequence(&mut seq)
                .returning(|| ());
            mock_path_store
                .expect_unwatch()
                .in_sequence(&mut seq)
                .with(predicate::eq(PathBuf::from(path.to_string())))
                .returning(|_| Ok(()));
            Ok(mock_path_store)
        });

        let backend = init_lsp_backend_default().await?;

        backend.unwatch_path(Path::new(path))?;

        Ok(())
    }

    #[tokio::test]
    async fn test_unwatch_path_failure_path_store_watch_failed() -> Result<()> {
        let path = "/path/to/watch";

        let ctx = MockPathStore::new_context();
        ctx.expect().returning(|_, _| {
            let mut seq = Sequence::new();
            let mut mock_path_store = MockPathStore::default();
            mock_path_store
                .expect_start_watcher()
                .in_sequence(&mut seq)
                .returning(|| ());
            mock_path_store
                .expect_unwatch()
                .in_sequence(&mut seq)
                .with(predicate::eq(PathBuf::from(path.to_string())))
                .returning(|_| Err(anyhow!("Failed to unwatch path")));
            Ok(mock_path_store)
        });

        let backend = init_lsp_backend_default().await?;

        assert_eq!(
            backend
                .unwatch_path(Path::new(path))
                .unwrap_err()
                .to_string(),
            "Failed to unwatch path"
        );

        Ok(())
    }
}
