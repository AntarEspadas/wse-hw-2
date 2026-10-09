use clap::Parser;

use std::error::Error;
use std::println;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use indexer::cli::Args;
use indexer::generate_index;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    generate_index(
        &args.input,
        args.chunk_size,
        args.workers,
        &args.out,
        args.write_txt,
        !args.skip_bin,
    )?;

    let finish = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    let elapsed = finish - start;

    println!("Took {:?}", elapsed);

    Ok(())
}
