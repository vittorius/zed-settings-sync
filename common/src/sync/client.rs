use crate::sync::LocalFileData;
use anyhow::{Context, Result};

use jsonc_parser::{ParseOptions, cst::CstRootNode, errors::ParseError};
use octocrab::{Error as OctocrabError, GitHubError};
use paths as zed_paths;
use thiserror::Error;
use tracing::{info, instrument};

#[derive(Debug)]
pub struct Client {
    octocrab: octocrab::Octocrab,
    gist_id: String,
    github_token: String,
}

impl Client {
    #[allow(clippy::missing_errors_doc)]
    pub fn new(gist_id: String, github_token: String) -> Result<Self> {
        let client = octocrab::Octocrab::builder()
            .personal_token(github_token.clone())
            .build()
            .with_context(|| "Failed to build the Github client")?;

        Ok(Self {
            octocrab: client,
            gist_id,
            github_token,
        })
    }

    #[allow(clippy::missing_errors_doc)]
    #[instrument(skip_all)]
    pub async fn sync_file(&self, data: LocalFileData) -> Result<(), FileError> {
        info!("Syncing file: {}", data.path.display());

        let body =
            Self::process_file_body_on_sync(&data.body, &data.path == zed_paths::settings_file())
                .map_err(|err| FileError::from_error(&data.filename, err))?;

        self.octocrab
            .gists()
            .update(&self.gist_id)
            .file(&data.filename)
            .with_content(&body)
            .send()
            .await
            .map_err(|err| FileError::from_error(&data.filename, err.into()))?;

        info!("File synced: {}", data.path.display());

        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    #[instrument(skip_all)]
    pub async fn load_files(
        &self,
    ) -> Result<impl IntoIterator<Item = Result<(String, String), FileError>>, Error> {
        Ok(self
            .octocrab
            .gists()
            .get(&self.gist_id)
            .await?
            .files
            .into_iter()
            .filter(|(file_name, file)| {
                file_name.eq_ignore_ascii_case(".json") && file.content.is_some()
            })
            .map(|(file_name, file)| {
                match Self::process_file_body_on_load(
                    #[allow(clippy::expect_used)]
                    &file
                        .content
                        .expect("File content presence was already verified"),
                    self.github_token.clone(),
                    file_name == *ZED_CONFIG_FILE_NAME, // TODO: extract to a lazy_static
                ) {
                    Ok(content) => Ok((file_name, content)),
                    Err(error) => Err(FileError { file_name, error }),
                }
            }))
    }

    fn process_file_body_on_sync(body: &str, is_settings_file: bool) -> Result<String, Error> {
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

    fn process_file_body_on_load(
        body: &str,
        github_token: String,
        is_settings_file: bool,
    ) -> Result<String, Error> {
        let root =
            CstRootNode::parse(body, &ParseOptions::default()).map_err(Error::InvalidJson)?;

        if is_settings_file {
            Self::unmask_auth_token(&root, github_token)?;
        }

        Ok(root.to_string())
    }

    fn unmask_auth_token(root: &CstRootNode, github_token: String) -> Result<String, Error> {
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
            .set_value(github_token.into());

        Ok(root.to_string())
    }
}

#[derive(Error, Debug)]
#[error("Error processing file {file_name}: {error}")]
pub struct FileError {
    file_name: String,
    error: Error,
}

impl FileError {
    fn from_error(file_name: impl Into<String>, error: Error) -> Self {
        Self {
            file_name: file_name.into(),
            error,
        }
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid JSON: {0}")]
    InvalidJson(ParseError),
    #[error("Invalid config structure: {0}")]
    InvalidConfig(String),
    #[error("Github error: {0}")]
    Github(GitHubError),
    #[error("Internal error: {0}")]
    Internal(Box<dyn std::error::Error + Send + Sync>),
    #[error("Unhandled internal error from underlying client library: {0}")]
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
