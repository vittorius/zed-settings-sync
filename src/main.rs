#[double]
use crate::file_loader::FileLoader;
use crate::std_interactive_io::StdInteractiveIO;
use anyhow::Result;
use clap::{Parser, Subcommand};
#[double]
use common::config::Config;
use common::interactive_io::InteractiveIO;
#[double]
use common::sync::GithubClient;
#[cfg(test)]
use common::{nextest_only, test_support::zed_paths};
use mockall_double::double;
#[cfg(not(test))]
use paths as zed_paths;

mod file_loader;
mod std_interactive_io;

#[derive(Debug, Parser)]
#[command(about = "Zed Settings Sync extension CLI tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Load all Zed user settings files from a gist
    Load {
        /// Force overwriting local settings files even if they exist
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let mut std_io = StdInteractiveIO;

    match args.command {
        Command::Load { force } => {
            load(&mut std_io, force).await?;
        }
    };

    std_io.write_line("🟢 All done.")?;

    Ok(())
}

async fn load<T: InteractiveIO + 'static>(io: &mut T, force: bool) -> Result<()> {
    let config = if zed_paths::settings_file().exists() {
        io.write_line("Loading settings from file")?;
        Config::from_settings_file()?
    } else {
        io.write_line("Zed settings file not found, probably you haven't installed Zed yet?")?;
        Config::from_interactive_io(io)?
    };

    let client = GithubClient::new(config.gist_id().into(), config.github_token().into())?;
    let mut loader = FileLoader::new(&client, io, force);

    loader.load_files().await
}

#[cfg(test)]
nextest_only!();

#[cfg(test)]
mod tests {
    use crate::file_loader::{
        __mock_MockFileLoader::__new::Context as MockFileLoaderNewContext, MockFileLoader,
    };
    use anyhow::Result;
    use assert_fs::prelude::*;
    use common::{
        config::MockConfig,
        interactive_io::MockInteractiveIO,
        sync::{
            __mock_MockGithubClient::__new::Context as MockGithubClientNewContext, Client,
            MockGithubClient,
        },
        test_support::zed_settings_file,
    };
    use mockall::{Sequence, predicate};

    use super::*;

    fn setup_interactive_io_mock(io: &mut MockInteractiveIO, seq: &mut Sequence) {
        io.expect_write_line()
            .in_sequence(seq)
            .returning(|_| Ok(()))
            .with(predicate::eq(
                "Zed settings file not found, probably you haven't installed Zed yet?",
            ));
    }

    // TODO: group all params into a struct
    fn setup_client_and_loader_mocks(
        seq: &mut Sequence,
        gh_client_ctx: &MockGithubClientNewContext,
        file_loader_ctx: &MockFileLoaderNewContext,
        force: bool,
        gist_id: Option<String>,
        github_token: Option<String>,
    ) {
        gh_client_ctx.expect().in_sequence(seq).returning(
            move |gist_id_received, github_token_received| {
                if let Some(ref gist_id_value) = gist_id {
                    assert_eq!(gist_id_value, &gist_id_received);
                }
                if let Some(ref github_token_value) = github_token {
                    assert_eq!(github_token_value, &github_token_received);
                }

                let mut mock_github_client = MockGithubClient::default();
                mock_github_client
                    .expect_id()
                    .return_const("mock_client_id".to_string());
                Ok(mock_github_client)
            },
        );

        file_loader_ctx
            .expect()
            .in_sequence(seq)
            .returning(move |client, _io, force_received| {
                // testing that FileLoader has received the correct client, configured from Config properties
                let mock_github_client: &MockGithubClient =
                    unsafe { &(*(client as *const dyn Client as *const MockGithubClient)) };
                assert_eq!(mock_github_client.id(), "mock_client_id");

                assert_eq!(force, force_received);

                let mut mock_file_loader = MockFileLoader::default();
                mock_file_loader.expect_load_files().returning(|| Ok(()));
                mock_file_loader
            });
    }

    #[tokio::test]
    async fn test_config_is_built_from_settings_file_if_it_exists() -> Result<()> {
        zed_settings_file().touch()?;

        let mut seq = Sequence::new();

        let mut io = MockInteractiveIO::default();
        io.expect_write_line()
            .in_sequence(&mut seq)
            .returning(|_| Ok(()))
            .with(predicate::eq("Loading settings from file"));

        let ctx = MockConfig::from_settings_file_context();
        ctx.expect().in_sequence(&mut seq).returning(|| {
            let mut mock_config = MockConfig::default();
            mock_config.expect_gist_id().return_const(String::default());
            mock_config
                .expect_github_token()
                .return_const(String::default());
            Ok(mock_config)
        });

        // we need to create contexts in the test function so they are not dropped before the test finishes
        let gh_ctx = MockGithubClient::new_context();
        let file_loader_ctx = MockFileLoader::new_context();
        setup_client_and_loader_mocks(&mut seq, &gh_ctx, &file_loader_ctx, false, None, None);

        load(&mut io, false).await
    }

    #[tokio::test]
    async fn test_config_is_built_from_user_input_if_settings_file_does_not_exist() -> Result<()> {
        let mut seq = Sequence::new();

        let mut io = MockInteractiveIO::default();
        setup_interactive_io_mock(&mut io, &mut seq);

        let ctx = MockConfig::from_interactive_io_context();
        ctx.expect()
            .in_sequence(&mut seq)
            .returning(|_io: &mut MockInteractiveIO| {
                let mut mock_config = MockConfig::default();
                mock_config.expect_gist_id().return_const(String::default());
                mock_config
                    .expect_github_token()
                    .return_const(String::default());
                Ok(mock_config)
            });

        // we need to create contexts in the test function so they are not dropped before the test finishes
        let gh_ctx = MockGithubClient::new_context();
        let file_loader_ctx = MockFileLoader::new_context();
        setup_client_and_loader_mocks(&mut seq, &gh_ctx, &file_loader_ctx, false, None, None);

        load(&mut io, false).await
    }

    #[tokio::test]
    async fn test_force_is_passed_to_file_loader() -> Result<()> {
        let mut seq = Sequence::new();

        let mut io = MockInteractiveIO::default();
        setup_interactive_io_mock(&mut io, &mut seq);

        let ctx = MockConfig::from_interactive_io_context();
        ctx.expect()
            .in_sequence(&mut seq)
            .returning(|_io: &mut MockInteractiveIO| {
                let mut mock_config = MockConfig::default();
                mock_config.expect_gist_id().return_const(String::default());
                mock_config
                    .expect_github_token()
                    .return_const(String::default());
                Ok(mock_config)
            });

        // we need to create contexts in the test function so they are not dropped before the test finishes
        let gh_ctx = MockGithubClient::new_context();
        let file_loader_ctx = MockFileLoader::new_context();
        setup_client_and_loader_mocks(&mut seq, &gh_ctx, &file_loader_ctx, true, None, None);

        load(&mut io, true).await
    }

    #[tokio::test]
    async fn test_params_from_config_are_passed_to_github_client() -> Result<()> {
        let gist_id = "1234567890";
        let github_token = "abcdefg";

        let mut seq = Sequence::new();

        let mut io = MockInteractiveIO::default();
        setup_interactive_io_mock(&mut io, &mut seq);

        let ctx = MockConfig::from_interactive_io_context();
        ctx.expect()
            .in_sequence(&mut seq)
            .returning(|_io: &mut MockInteractiveIO| {
                let mut mock_config = MockConfig::default();
                mock_config
                    .expect_gist_id()
                    .return_const(gist_id.to_string());
                mock_config
                    .expect_github_token()
                    .return_const(github_token.to_string());
                Ok(mock_config)
            });

        // we need to create contexts in the test function so they are not dropped before the test finishes
        let gh_ctx = MockGithubClient::new_context();
        let file_loader_ctx = MockFileLoader::new_context();
        setup_client_and_loader_mocks(
            &mut seq,
            &gh_ctx,
            &file_loader_ctx,
            false,
            Some(gist_id.to_string()),
            Some(github_token.to_string()),
        );

        load(&mut io, false).await
    }
}
