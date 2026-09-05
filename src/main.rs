mod cli;
mod core;
mod croft;
mod regent;
mod store;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
  cli::run()
}
