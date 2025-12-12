use tower_lsp::{LspService, Server};
use tracing::info;

use crate::backend::Backend;

mod app_state;
mod backend;
mod logger;
mod watching;

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
