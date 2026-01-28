mod client;
mod codespan;
mod commands;
mod config;
mod validator;

use clap::Parser;
use color_eyre::Result;
pub use commands::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command() {
        Command::Start(cmd) => cmd.run().await?,
    };

    Ok(())
}
