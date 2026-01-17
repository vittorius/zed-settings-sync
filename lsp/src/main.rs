#[cfg(not(test))]
use tower_lsp::{LspService, Server};
use tracing::info;

#[cfg(not(test))]
use crate::backend::Backend;

mod app_state;
mod backend;
mod logger;
#[cfg(test)]
mod mocks;
mod watching;

#[cfg(test)]
test_support::nextest_only!();

#[tokio::main]
async fn main() {
    logger::init_logger();

    info!(
        "Starting Zed Settings Sync LSP server v{}",
        env!("CARGO_PKG_VERSION")
    );

    #[cfg(not(test))] // to avoid "type mismatch in function arguments" error in LspService::new
    let (service, socket) = LspService::new(Backend::new);

    info!("LSP service created, starting server");

    #[cfg(not(test))]
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;

    info!("Zed Settings Sync LSP server stopped");
}
