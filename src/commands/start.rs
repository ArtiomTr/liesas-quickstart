use std::path::PathBuf;

use clap::Args;
use color_eyre::{Result, eyre::Context as _};
use tokio::{fs::File, io::AsyncReadExt};

use crate::{
    codespan::{report_config_error, report_toml_error},
    config::NetworkConfig,
};

#[derive(Debug, Clone, Args)]
pub struct StartCommand {
    #[arg(long)]
    config: PathBuf,
}

impl StartCommand {
    pub async fn run(&self) -> Result<()> {
        let mut file = File::open(&self.config)
            .await
            .context(format!("failed to read config at {:?}", self.config))?;

        let mut buffer = String::new();
        file.read_to_string(&mut buffer)
            .await
            .context("invalid network config")?;

        let config: NetworkConfig = match toml::de::from_str(&buffer) {
            Ok(value) => value,
            Err(err) => report_toml_error(
                "Invalid network configuration".to_owned(),
                self.config.clone(),
                buffer,
                err,
            ),
        };

        let resolved = match config.resolve() {
            Ok(value) => value,
            Err(err) => report_config_error(self.config.clone(), buffer, err),
        };

        Ok(())
    }
}
