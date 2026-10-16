use std::error::Error;

use clap::Parser;
use merger::{cli::Args, merge};

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    merge(
        args.input_dir.as_ref(),
        args.out.as_ref(),
        args.input_count,
        args.buffer_size,
    )?;

    Ok(())
}
