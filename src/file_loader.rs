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
    use common::sync::{Error, FileResult};

    // TODO: use if applicable
    fn empty_iter() -> Result<Box<dyn Iterator<Item = FileResult>>, Error> {
        Ok(Box::new([].into_iter()))
    }

    async fn test_non_existing_file_is_written() -> Result<()> {
        todo!()
    }

    async fn test_existing_file_is_written_if_confirmed() -> Result<()> {
        todo!()
    }

    async fn test_existing_file_is_written_if_not_confirmed() -> Result<()> {
        todo!()
    }
}
