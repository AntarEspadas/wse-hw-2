use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Directory containing index files (index.bin, lexicon.bin)
    #[arg(short, long, default_value_t = "./output".to_string())]
    pub index_dir: String,

    /// Number of top results to be shown for each query
    #[arg(short, long, default_value_t = 10)]
    pub top_results: usize,

    /// Cutoff score for results
    #[arg(short, long, default_value_t = 5f32)]
    pub score_threshold: f32,
}
