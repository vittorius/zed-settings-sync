use anyhow::{Context, Result};
use notify::Event;
use tracing::{info, instrument};

#[derive(Debug)]
pub struct Client {
    _client: octocrab::Octocrab,
}

impl Client {
    pub fn new(personal_token: &str) -> Result<Self> {
        let client = octocrab::Octocrab::builder()
            .personal_token(personal_token)
            .build()
            .with_context(|| "Failed to build the Github client")?;
        Ok(Self { _client: client })
    }
}

impl Client {
    #[instrument(skip(self, event))]
    pub async fn notify(&self, event: Event) -> Result<()> {
        // TODO: replace dummy async block with the REST client call
        async {
            info!("event received: {event:?}");

            Ok(())
        }
        .await
    }
}
