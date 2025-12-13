use crate::sync::LocalFileData;
use anyhow::{Context, Result};
use itertools::Itertools;
use jsonc_parser::{ParseOptions, cst::CstRootNode, errors::ParseError};
use octocrab::{Error as OctocrabError, GitHubError};
use paths as zed_paths;
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
    pub async fn sync_file(&self, data: LocalFileData) -> Result<(), Error> {
        info!("Syncing file: {}", data.path.display());

        let body =
            Self::process_file_body_on_sync(&data.body, &data.path == zed_paths::settings_file())?;

        self.octocrab
            .gists()
            .update(&self.gist_id)
            .file(&data.filename)
            .with_content(&body)
            .send()
            .await?;

        info!("File synced: {}", data.path.display());

        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    #[instrument(skip_all)]
    pub async fn load_files(&self) -> Result<impl IntoIterator<Item = (String, String)>, Error> {
        self.octocrab
            .gists()
            .get(&self.gist_id)
            .await?
            .files
            .into_iter()
            .filter(|(file_name, file)| {
                file_name.eq_ignore_ascii_case(".json") && file.content.is_some()
            })
            .map(|(file_name, file)| {
                let content = Self::process_file_body_on_load(
                    #[allow(clippy::expect_used)]
                    &file
                        .content
                        .expect("File content presence was already verified"),
                    self.github_token.clone(),
                    file_name == "settings.json", // TODO: extract to a lazy_static
                )?;

                Ok((file_name, content))
            })
            .process_results(|iter| iter.collect::<Vec<_>>())
        // TODO: tried to just .process_results(|iter| iter) to avoid allocation but got
        // .process_results(|iter| iter)
        //    |                               ----- ^^^^ returning this value requires that `'1` must outlive `'2`
        //    |                               |   |
        //    |                               |   return type of closure is ProcessResults<'2, std::iter::Map<std::iter::Filter<std::collections::btree_map::IntoIter<std::string::String, GistFile>, {closure@common/src/sync/client.rs:62:21: 62:40}>, {closure@common/src/sync/client.rs:65:18: 65:37}>, client::Error>
        //    |                               has type `ProcessResults<'1, std::iter::Map<std::iter::Filter<std::collections::btree_map::IntoIter<std::string::String, GistFile>, {closure@common/src/sync/client.rs:62:21: 62:40}>, {closure@common/src/sync/client.rs:65:18: 65:37}>, client::Error>`
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

    fn process_file_body_on_load(body: &str, github_token: String, is_settings_file: bool) -> Result<String, Error> {
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

#[derive(Debug)]
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
