use std::{error::Error, path::Path};

use merger::merge;

const BUFFER_SIZE: usize = 1000;
const IN_FOLDER: &str = "F:\\output";
// const OUT_PATH: &str = "F:\\output\\final";
const CHUNKS: usize = 4;

fn main() -> Result<(), Box<dyn Error>> {
    let work_folder = Path::new(IN_FOLDER);

    merge(work_folder, work_folder, CHUNKS, BUFFER_SIZE)?;

    Ok(())
}
