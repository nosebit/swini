mod cli;
mod core;
mod croft;
mod drover;
mod pig;
mod regent;
mod store;
mod supervisor;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
  cli::run()
}
