mod start;

use clap::{Parser, Subcommand};

use crate::commands::start::StartCommand;

#[derive(Debug, Clone, Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    start: Option<StartCommand>,
}

#[derive(Debug, Clone, Subcommand)]
#[command(args_conflicts_with_subcommands = true)]
pub enum Command {
    Start(StartCommand),
}

impl Cli {
    pub fn command(&self) -> Command {
        self.command
            .clone()
            .or(self.start.clone().map(Command::Start))
            .expect("clap should automatically handle default command")
    }
}
