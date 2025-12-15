use crate::{
    ZED_CONFIG_FILE_NAME,
    sync::{Error, LocalFileData},
};
use anyhow::{Context, Result};

use jsonc_parser::{ParseOptions, cst::CstRootNode};
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
                file_name.to_lowercase().ends_with(".json") && file.content.is_some()
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
