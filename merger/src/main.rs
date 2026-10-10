use std::{error::Error, path::Path};

use merger::merge;

const BUFFER_SIZE: usize = 1000;
const IN_FOLDER: &str = "F:\\output";
// const OUT_PATH: &str = "F:\\output\\final";
const CHUNKS: usize = 2;

fn main() -> Result<(), Box<dyn Error>> {
    let work_folder = Path::new(IN_FOLDER);

    let (index_path, lexicon_path) = merge(work_folder, CHUNKS, BUFFER_SIZE)?;

    println!("Index written to {index_path:?}");
    println!("Lexicon written to {lexicon_path:?}");

    Ok(())
}
