use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Input TSV file
    #[arg(default_value_t = "./data/collection.tsv".to_string())]
    pub input: String,

    /// Output folder for files
    #[arg(short, long, default_value_t = "./output/".to_string())]
    pub out: String,

    /// Number of worker threads
    #[arg(short, long, default_value_t = 8)]
    pub workers: usize,

    /// Chunk size in bytes
    #[arg(short, long, default_value_t = 100_000_000)]
    pub chunk_size: usize,

    /// Enables writing output in plaintext format
    #[arg(long)]
    pub write_txt: bool,

    /// Disables writing output in binary format
    #[arg(long)]
    pub skip_bin: bool,
}
