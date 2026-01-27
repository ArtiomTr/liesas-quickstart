use std::path::PathBuf;

use clap::Args;
use color_eyre::{Result, eyre::Context as _};
use tokio::{fs::File, io::AsyncReadExt};

use crate::config::NetworkConfig;

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

        let config: NetworkConfig =
            toml::de::from_str(&buffer).context("invalid network config")?;

        let resolved = config.resolve()?;

        Ok(())
    }
}
