use crate::sync::file_data::FileData;
use anyhow::{Context, Result};
use jsonc_parser::{ParseOptions, cst::CstRootNode, errors::ParseError};
use octocrab::{Error as OctocrabError, GitHubError};
use tracing::{info, instrument};

// TODO: extract Client to a shared module to be used by both LSP and CLI tool crates
#[derive(Debug)]
pub struct Client {
    client: octocrab::Octocrab,
    gist_id: String,
}

impl Client {
    pub fn new(gist_id: String, github_token: String) -> Result<Self> {
        let client = octocrab::Octocrab::builder()
            .personal_token(github_token)
            .build()
            .with_context(|| "Failed to build the Github client")?;

        Ok(Self { client, gist_id })
    }

    #[instrument(skip_all)]
    pub async fn sync_file(&self, data: FileData) -> Result<(), Error> {
        info!("Syncing file: {}", data.path.display());

        let body = Self::process_file_body(&data.body, &data.path == zed_paths::settings_file())?;

        self.client
            .gists()
            .update(&self.gist_id)
            .file(&data.filename)
            .with_content(&body)
            .send()
            .await?;

        info!("File synced: {}", data.path.display());

        Ok(())
    }

    fn process_file_body(body: &str, is_settings_file: bool) -> Result<String, Error> {
        let root =
            CstRootNode::parse(body, &ParseOptions::default()).map_err(Error::InvalidJson)?;

        if is_settings_file {
            Self::mask_auth_token(&root)?;
        }

        Ok(root.to_string())
    }

    fn mask_auth_token(root: &CstRootNode) -> Result<(), Error> {
        let root_obj = root.object_value_or_set();
        root_obj
            .get("lsp")
            .ok_or(Error::InvalidConfig(r#"Missing "lsp" key"#.to_string()))?
            .object_value()
            .ok_or(Error::InvalidConfig(
                r#"Missing "lsp" configuration object"#.to_string(),
            ))?
            .get("settings_sync")
            .ok_or(Error::InvalidConfig(
                r#"Missing "settings_sync" key"#.to_string(),
            ))?
            .object_value()
            .ok_or(Error::InvalidConfig(
                r#"Missing "settings_sync" configuration object"#.to_string(),
            ))?
            .get("initialization_options")
            .ok_or(Error::InvalidConfig(
                r#"Missing "initialization_options" key"#.to_string(),
            ))?
            .object_value()
            .ok_or(Error::InvalidConfig(
                r#"Missing "initialization_options" configuration object"#.to_string(),
            ))?
            .get("github_token")
            .ok_or(Error::InvalidConfig("Missing github_token".to_string()))?
            .set_value("[masked]".into());

        Ok(())
    }
}

pub enum Error {
    InvalidJson(ParseError),
    InvalidConfig(String),
    Github(GitHubError),
    Internal(Box<dyn std::error::Error + Send + Sync>),
    UnhandledInternal(String),
}

impl From<OctocrabError> for Error {
    fn from(err: OctocrabError) -> Self {
        match err {
            OctocrabError::GitHub { source, .. } => Error::Github(*source),
            OctocrabError::UriParse { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Uri { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Installation { .. } => Error::Internal(Box::new(err)),
            OctocrabError::InvalidHeaderValue { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Http { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::InvalidUtf8 { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Encoder { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Service { source, .. } | OctocrabError::Other { source, .. } => {
                Error::Internal(source)
            }
            OctocrabError::Hyper { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::SerdeUrlEncoded { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Serde { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::Json { source, .. } => Error::Internal(Box::new(source)),
            OctocrabError::JWT { source, .. } => Error::Internal(Box::new(source)),
            _ => Error::UnhandledInternal(format!(
                "Unhandled Octocrab error from non-exhaustive match, ping author to update deps: {err:?}"
            )),
        }
    }
}
