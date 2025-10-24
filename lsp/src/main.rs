use std::fs;
use std::io::{BufReader, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use interprocess::local_socket::traits::Stream;
use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName};
use tokio::task::JoinError;
use tower_lsp::jsonrpc::Result;
use tower_lsp::{
    Client, LanguageServer, LspService, Server,
    lsp_types::{
        InitializeParams, InitializeResult, InitializedParams, MessageType, SaveOptions,
        ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncOptions,
        TextDocumentSyncSaveOptions, WorkspaceServerCapabilities,
    },
};
use tracing::{debug, error, info, instrument};

use crate::service::AppState;

mod logger;
mod service;

const EXTENSION_ID: &str = "settings-sync";
const LSP_BINARY: &str = "zed-settings-sync-lsp";
const WATCHER_BINARY: &str = "zed-settings-sync-watcher";
const WATCHER_SOCKET_NAME: &str = "zed-settings-sync-watcher.sock";

#[derive(Debug)]
struct Backend {
    client: Client,
    _app_state: Arc<AppState>,
    // TODO: add GistService
}

impl Backend {
    fn new(client: Client) -> Self {
        let app_state = Arc::new(AppState::new());

        info!("Backend initialized");

        Self {
            client,
            _app_state: app_state,
        }
    }

    // async fn on_change(&self, uri: &tower_lsp::lsp_types::Url) {
    //     debug!("Document changed");

    //     let doc = {
    //         let workspace = self.app_state.workspace.lock().await;
    //         let workspace_path = Path::new(workspace.path().unwrap_or(""));

    //         Document::new(uri, workspace_path)
    //     };

    //     // let client = RestClient::new();
    //     // let _ = client
    //     //     .post("https://webhook.site/a2bbd754-c865-4328-96cd-a7f2a070b971")
    //     //     .json(&json!({"uri": uri.to_string()}))
    //     //     .send()
    //     //     .await;

    //     if let Err(e) = self.presence_service.update_presence(Some(doc)).await {
    //         error!("Failed to update presence: {}", e);
    //     } else {
    //         debug!("Presence updated successfully");
    //     }
    // }

    fn resolve_workspace_path(params: &InitializeParams) -> PathBuf {
        if let Some(folders) = &params.workspace_folders
            && let Some(first_folder) = folders.first()
        {
            let path = Path::new(first_folder.uri.path()).to_owned();
            debug!("Using workspace folder: {}", path.display());
            return path;
        }

        let root_uri = params.root_uri.as_ref().expect(
            "Failed to get workspace path - neither workspace_folders nor root_uri is present",
        );

        let path = Path::new(root_uri.path()).to_owned();
        debug!("Using root URI: {}", path.display());
        path
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("Initializing Zed Settings Sync LSP");
        info!("init params: {params:?}");

        self.client
            .log_message(MessageType::INFO, format!("init params: {params:?}"))
            .await;

        // Resolve workspace
        let workspace_path = Self::resolve_workspace_path(&params);
        info!("Workspace path: {}", workspace_path.display());

        // let socket_file = extension_work_dir.join(WATCHER_SOCKET_FILE);

        // TODO: improve
        let Ok(socket_name) = WATCHER_SOCKET_NAME.to_ns_name::<GenericNamespaced>() else {
            error!(
                "Failed to create a local socket name from: {}",
                WATCHER_SOCKET_NAME
            );
            return Err(tower_lsp::jsonrpc::Error::internal_error());
        };
        let mut buffer = String::with_capacity(128);
        
        // check if watcher is already running
        let processes = match sysinfo::System::new_all().processes() {
            processes => processes.values()
                .filter(|process| process.name().ends_with("zed-settings-sync-watcher"))
                .count() > 0,
        };

        if !processes {
            // connecting to socket failed, the watcher service must be not spawned yet
            info!("Watcher process not found, starting new instance");
        }

        let Ok(conn) = Stream::connect(socket_name) else {
            // connecting to socket failed, the watcher service must be not spawned yet

            let extension_work_dir = zed_paths::extensions_dir().join("work").join(EXTENSION_ID);
            let watcher_binary_path = extension_work_dir.join(WATCHER_BINARY);
            let mut cmd = Command::new(watcher_binary_path.as_os_str());
            if let Err(e) = cmd.spawn() {
                error!("Failed to start the watcher service: {}", e);
                return Err(tower_lsp::jsonrpc::Error::internal_error());
            }
            Stream::connect(socket_name).expect("Must connect")
        };
        // Wrap it into a buffered reader right away so that we could receive a single line out of it.
        let mut conn = BufReader::new(conn);

        // {
        //     let mut workspace = self.app_state.workspace.lock().await;
        //     if let Err(e) = workspace.set_workspace(&workspace_path) {
        //         error!("Failed to set workspace: {}", e);
        //         return Err(tower_lsp::jsonrpc::Error::internal_error());
        //     }
        //     info!("Workspace set to: {}", workspace.name());
        // }

        // Set git remote URL
        // {
        //     let mut git_remote_url = self.app_state.git_remote_url.lock().await;
        //     let remote_url = get_repository_and_remote(workspace_path.to_str().unwrap_or(""));

        //     if let Some(ref url) = remote_url {
        //         info!("Git remote URL found: {}", url);
        //     } else {
        //         debug!("No git remote URL found");
        //     }

        //     *git_remote_url = remote_url;
        // }

        // Update config
        // {
        //     let mut config = self.app_state.config.lock().await;
        //     if let Err(e) = config.update(params.initialization_options) {
        //         error!("Failed to update config: {}", e);
        //         return Err(tower_lsp::jsonrpc::Error::internal_error());
        //     }

        //     debug!(
        //         "Configuration updated: application_id={}, git_integration={}",
        //         config.application_id, config.git_integration
        //     );

        //     // Check if workspace is suitable
        //     if !config.rules.suitable(workspace_path.to_str().unwrap_or("")) {
        //         info!("Workspace not suitable according to rules, exiting");
        //         exit(0);
        //     }
        // }

        // Initialize Discord
        // {
        //     let config = self.app_state.config.lock().await;
        //     if let Err(e) = self
        //         .presence_service
        //         .initialize_discord(&config.application_id)
        //         .await
        //     {
        //         error!("Failed to initialize Discord: {}", e);
        //         return Err(tower_lsp::jsonrpc::Error::internal_error());
        //     }
        //     info!("Discord client initialized and connected");
        // }

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
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

        self.client
            .log_message(
                MessageType::INFO,
                "Zed Settings Sync LSP server initialized! Woo-hoo!",
            )
            .await;
    }

    #[instrument(skip(self))]
    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Zed Settings Sync LSP");

        // probably no services to shutdown as GistService will be a simple REST client
        // if let Err(e) = self.presence_service.shutdown().await {
        //     error!("Failed to shutdown presence service: {}", e);
        // } else {
        //     info!("Presence service shutdown successfully");
        // }

        Ok(())
    }

    // #[instrument(skip(self, params))]
    // async fn did_open(&self, params: DidOpenTextDocumentParams) {
    //     debug!("Document opened: {}", params.text_document.uri);
    //     self.on_change(&params.text_document.uri).await;
    // }

    // // #[instrument(skip(self, params))]
    // #[instrument]
    // async fn did_change(&self, params: DidChangeTextDocumentParams) {
    //     debug!("Document changed: {}", params.text_document.uri);
    //     // warn!(
    //     //     "Text of the changed document: {}",
    //     //     params
    //     //         .content_changes
    //     //         .iter()
    //     //         .map(|ch| ch.text.clone())
    //     //         .reduce(|acc, e| { format!("{acc}; {e}") })
    //     //         .unwrap_or("<none>".to_string())
    //     // );
    //     self.on_change(&params.text_document.uri).await;
    // }

    // // #[instrument(skip(self, params))]
    // #[instrument]
    // async fn did_save(&self, params: DidSaveTextDocumentParams) {
    //     debug!("Document saved: {}", params.text_document.uri);
    //     warn!(
    //         "Text of the saved document: {}",
    //         params.text.unwrap_or("<none>".to_string())
    //     );

    //     self.on_change(&params.text_document.uri).await;
    // }

    // #[instrument]
    // async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
    //     warn!("workspace/didChangeConfiguration received: {:?}", params);
    // }
}
#[tokio::main]
async fn main() {
    logger::init_logger();

    info!(
        "Starting Zed Settings Sync LSP server v{}",
        env!("CARGO_PKG_VERSION")
    );

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);

    info!("LSP service created, starting server");
    Server::new(stdin, stdout, socket).serve(service).await;

    info!("Zed Settings Sync LSP server stopped");
}
