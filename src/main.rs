mod cli;
mod core;
mod store;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  core::telemetry::init();

  cli::run().await
}
