use crate::{
    ZED_CONFIG_FILE_NAME,
    sync::{Client, Error, FileError, FileResult, LocalFileData},
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use jsonc_parser::{ParseOptions, cst::CstRootNode};
use mockall::mock;
use paths as zed_paths;
use tracing::{info, instrument};

#[derive(Debug)]
pub struct GithubClient {
    octocrab: octocrab::Octocrab,
    gist_id: String,
    github_token: String,
}

#[async_trait]
impl Client for GithubClient {
    #[instrument(skip_all)]
    async fn sync_file(&self, data: LocalFileData) -> Result<(), FileError> {
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

    #[instrument(skip_all)]
    async fn load_files<'a>(&'a self) -> Result<Box<dyn Iterator<Item = FileResult> + 'a>, Error> {
        Ok(Box::new(
            self.octocrab
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
                        file_name == *ZED_CONFIG_FILE_NAME,
                    ) {
                        Ok(content) => Ok((file_name, content)),
                        Err(error) => Err(FileError { file_name, error }),
                    }
                }),
        ))
    }
}

impl GithubClient {
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

// currently, GithubClient is the only implementation of Client, so mocking it directly
mock! {
    pub GithubClient {
        pub fn id(&self) -> String; // for identity tracking in tests
        pub fn new(gist_id: String, github_token: String) -> Result<Self>;
    }

    #[async_trait]
    impl Client for GithubClient {
        async fn sync_file(&self, data: LocalFileData)
            -> Result<(), FileError>;

        async fn load_files<'a>(&'a self)
            -> Result<Box<dyn Iterator<Item = FileResult> + 'a>, Error>;
    }

    // impl fmt::Debug for GithubClient {
    //     fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
    //         f.debug_struct("GithubClient")
    //             .finish()
    //     }
    // }

    // impl PartialEq for GithubClient {
    //     fn eq(&self, other: &Self) -> bool {
    //         self.gist_id == other.gist_id && self.github_token == other.github_token
    //     }
    // }

    // impl Eq for GithubClient {}

}
