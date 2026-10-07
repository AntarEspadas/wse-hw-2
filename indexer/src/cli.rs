use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Input TSV file
    pub input: PathBuf,

    /// Output folder for files
    #[arg(short, long)]
    pub out: PathBuf,

    /// Number of worker threads
    #[arg(short, long)]
    pub workers: usize,

    /// Chunk size in bytes
    #[arg(short, long)]
    pub chunk_size: usize,
}
