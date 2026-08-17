use clap::{Parser, Subcommand};
use std::error::Error;

mod daemon;

#[derive(Parser, Debug)]
#[command(author, version, about = "Swini Command Line Interface", long_about = None)]
struct Cli {
  #[command(subcommand)]
  pub command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
  Daemon {
    #[command(subcommand)]
    command: daemon::Command,
  },
}

pub async fn run() -> Result<(), Box<dyn Error>> {
  let cli = Cli::parse();

  match cli.command {
    Command::Daemon { command } => daemon::run(command).await,
  }
}
