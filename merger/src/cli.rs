use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Directory containing partial index files
    #[arg(default_value_t = "./output".to_string())]
    pub input_dir: String,

    /// Directory where merged index files will be written
    #[arg(short, long, default_value_t = "./output".to_string())]
    pub out: String,

    /// Number of partial index files that will be simultaneously opened for merging
    #[arg(short, long, default_value_t = 4)]
    pub input_count: usize,

    /// Number of index entries that will be kept in memory for each file
    #[arg(short, long, default_value_t = 100)]
    pub buffer_size: usize,
}
