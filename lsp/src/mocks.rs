use std::fmt;

use mockall::mock;
use tower_lsp::lsp_types::MessageType;

mock! {
    pub LspClient {
        pub fn show_message(&self, msg_type: MessageType, message: String) -> impl Future<Output = ()> + Send + Sync;
    }

    impl Clone for LspClient {
        fn clone(&self) -> Self {
            Self::default()
        }
    }
}

impl fmt::Debug for MockLspClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LspClient").finish()
    }
}
