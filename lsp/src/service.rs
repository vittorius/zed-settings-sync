#[derive(Debug)]
pub struct AppState {
    // pub discord: Arc<Mutex<Discord>>,
    // pub config: Arc<Mutex<Configuration>>,
    // pub workspace: Arc<Mutex<WorkspaceService>>,
    // pub git_remote_url: Arc<Mutex<Option<String>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {}
    }
}
