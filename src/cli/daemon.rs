use clap::Subcommand;
use std::error::Error;

#[derive(Subcommand, Debug)]
pub enum Command {
  /// Starts the daemon in the foreground by default.
  Start {
    /// Run daemon in the background
    #[arg(short, long)]
    detached: bool,
  },
}

pub async fn run(cmd: Command) -> Result<(), Box<dyn Error>> {
  match cmd {
    Command::Start { detached } => {
      tracing::info!("Start the daemon");
    }
  }

  Ok(())
}
