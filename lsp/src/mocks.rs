use std::fmt;

use mockall::mock;
use tower_lsp::lsp_types::MessageType;

mock! {
    pub LspClient {
        pub fn show_message(&self, msg_type: MessageType, message: String) -> impl Future<Output = ()> + Send + Sync;
    }

    impl fmt::Debug for LspClient {
        fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
            f.debug_struct("LspClient").finish()
        }
    }

    impl Clone for LspClient {
        fn clone(&self) -> Self {
            Self::default()
        }
    }
}
