use std::fs;

use anyhow::Result;
use common::{interactive_io::InteractiveIO, sync::Client};
#[cfg(not(test))]
use paths as zed_paths;
#[cfg(test)]
use test_support::zed_paths;

pub struct FileLoader<'a> {
    client: &'a dyn Client,
    io: &'a mut dyn InteractiveIO,
    force: bool,
}

impl<'a> FileLoader<'a> {
    pub fn new(client: &'a dyn Client, io: &'a mut dyn InteractiveIO, force: bool) -> Self {
        FileLoader { client, io, force }
    }

    pub async fn load_files(&mut self) -> Result<()> {
        for file_load_result in self.client.load_files().await? {
            match file_load_result {
                Ok((file_name, content)) => self.process_loaded_file(file_name, content)?,
                Err(e) => self.io.write_line(&format!("🔴 {}", e))?,
            }
        }

        Ok(())
    }

    fn process_loaded_file(&mut self, file_name: String, content: String) -> Result<()> {
        let file_path = zed_paths::config_dir().join(&file_name);

        if file_path.exists() && !self.force {
            self.io
                .write(&format!("🟡 {file_name} exists, overwrite (y/n)? "))?;

            let mut answer = String::new();
            self.io.read_line(&mut answer)?;

            if answer.trim().to_lowercase().starts_with('y') {
                self.io.write_line(&format!("Overwriting {file_name}..."))?;
            } else {
                self.io.write_line(&format!("Skipping {file_name}"))?;
                return Ok(());
            }
        }

        fs::write(file_path, content)?;

        self.io.write_line(&format!("Written {file_name}"))?;

        Ok(())
    }
}

#[cfg(test)]
mockall::mock! {
    pub FileLoader {
        pub fn new<'a>(client: &'a dyn Client, io: &'a mut dyn InteractiveIO, force: bool) -> Self;
        pub async fn load_files(&mut self) -> Result<()>;
    }
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    use anyhow::Result;
    use assert_fs::prelude::*;
    use common::{
        interactive_io::MockInteractiveIO,
        sync::{Error, FileError, MockGithubClient},
    };
    use mockall::{Sequence, predicate};
    use test_support::zed_config_dir;

    use super::*;

    #[tokio::test]
    async fn test_non_existing_file_is_written() -> Result<()> {
        let mut seq = Sequence::new();

        let mut mock_client = MockGithubClient::default();
        mock_client
            .expect_load_files()
            .in_sequence(&mut seq)
            .returning(|| {
                Ok(Box::new(
                    [Ok(("tasks.json".to_string(), "content".to_string()))].into_iter(),
                ))
            });

        let mut mock_io = MockInteractiveIO::default();
        mock_io
            .expect_write_line()
            .in_sequence(&mut seq)
            .with(predicate::eq("Written tasks.json"))
            .returning(|_| Ok(()))
            .once();

        let mut file_loader = FileLoader::new(&mock_client, &mut mock_io, false);
        file_loader.load_files().await?;

        zed_config_dir().child("tasks.json").assert("content");

        Ok(())
    }

    #[tokio::test]
    async fn test_existing_file_is_written_if_confirmed() -> Result<()> {
        zed_config_dir()
            .child("settings.json")
            .write_str("existing content")?;

        let mut seq = Sequence::new();

        let mut mock_client = MockGithubClient::default();
        mock_client
            .expect_load_files()
            .in_sequence(&mut seq)
            .returning(|| {
                Ok(Box::new(
                    [Ok((
                        "settings.json".to_string(),
                        "{ \"key\": 1 }".to_string(),
                    ))]
                    .into_iter(),
                ))
            });

        let mut mock_io = MockInteractiveIO::default();
        mock_io
            .expect_write()
            .in_sequence(&mut seq)
            .with(predicate::eq("🟡 settings.json exists, overwrite (y/n)? "))
            .returning(|_| Ok(()))
            .once();
        mock_io
            .expect_read_line()
            .in_sequence(&mut seq)
            .once()
            .returning(|answer| {
                answer.push_str("YEZZZ\n");
                Ok(6)
            })
            .once();
        mock_io
            .expect_write_line()
            .in_sequence(&mut seq)
            .with(predicate::eq("Overwriting settings.json..."))
            .returning(|_| Ok(()))
            .once();
        mock_io
            .expect_write_line()
            .in_sequence(&mut seq)
            .with(predicate::eq("Written settings.json"))
            .returning(|_| Ok(()))
            .once();

        let mut file_loader = FileLoader::new(&mock_client, &mut mock_io, false);
        file_loader.load_files().await?;

        zed_config_dir()
            .child("settings.json")
            .assert("{ \"key\": 1 }");

        Ok(())
    }

    #[tokio::test]
    async fn test_existing_file_is_not_written_if_not_confirmed() -> Result<()> {
        zed_config_dir().child("keymap.json").write_str("{}")?;

        let mut seq = Sequence::new();

        let mut mock_client = MockGithubClient::default();
        mock_client
            .expect_load_files()
            .in_sequence(&mut seq)
            .returning(|| {
                Ok(Box::new(
                    [Ok((
                        "keymap.json".to_string(),
                        "{ \"foo\": \"bar\" }".to_string(),
                    ))]
                    .into_iter(),
                ))
            });

        let mut mock_io = MockInteractiveIO::default();
        mock_io
            .expect_write()
            .in_sequence(&mut seq)
            .with(predicate::eq("🟡 keymap.json exists, overwrite (y/n)? "))
            .returning(|_| Ok(()))
            .once();
        mock_io
            .expect_read_line()
            .in_sequence(&mut seq)
            .once()
            .returning(|answer| {
                answer.push_str("NOOO\n");
                Ok(5)
            })
            .once();
        mock_io
            .expect_write_line()
            .in_sequence(&mut seq)
            .with(predicate::eq("Skipping keymap.json"))
            .returning(|_| Ok(()))
            .once();

        let mut file_loader = FileLoader::new(&mock_client, &mut mock_io, false);
        file_loader.load_files().await?;

        zed_config_dir().child("keymap.json").assert("{}");

        Ok(())
    }

    #[tokio::test]
    async fn test_existing_file_is_written_if_forced() -> Result<()> {
        zed_config_dir()
            .child("debug.json")
            .write_str("existing content")?;

        let mut seq = Sequence::new();

        let mut mock_client = MockGithubClient::default();
        mock_client
            .expect_load_files()
            .in_sequence(&mut seq)
            .returning(|| {
                Ok(Box::new(
                    [Ok(("debug.json".to_string(), "{ \"key\": 1 }".to_string()))].into_iter(),
                ))
            });

        let mut mock_io = MockInteractiveIO::default();
        mock_io
            .expect_write_line()
            .in_sequence(&mut seq)
            .with(predicate::eq("Written debug.json"))
            .returning(|_| Ok(()))
            .once();

        let mut file_loader = FileLoader::new(&mock_client, &mut mock_io, true);
        file_loader.load_files().await?;

        zed_config_dir()
            .child("debug.json")
            .assert("{ \"key\": 1 }");

        Ok(())
    }

    #[tokio::test]
    async fn test_file_error_reporting() -> Result<()> {
        let mut seq = Sequence::new();

        let mut mock_client = MockGithubClient::default();
        let build_error = || {
            FileError::from_error(
                "tasks.json",
                Error::UnhandledInternal("Unhandled internal error".to_string()),
            )
        };
        mock_client
            .expect_load_files()
            .in_sequence(&mut seq)
            .returning(move || Ok(Box::new([Err(build_error())].into_iter())))
            .once();

        let mut mock_io = MockInteractiveIO::default();
        mock_io
            .expect_write_line()
            .in_sequence(&mut seq)
            .with(predicate::eq(format!("🔴 {}", build_error())))
            .returning(|_| Ok(()))
            .once();

        let mut file_loader = FileLoader::new(&mock_client, &mut mock_io, false);
        file_loader.load_files().await
    }
}
