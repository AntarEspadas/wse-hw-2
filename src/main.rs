use crossbeam_channel::bounded;
use std::error::Error;
use wse_hw_2::data::OutMessage;

use wse_hw_2::data::InMessage;
use wse_hw_2::produce_from_csv;
use wse_hw_2::worker;

const OUT_FOLDER: &str = "output";
const IN_PATH: &str = "./data/collection.tsv";
const CHUNK_SIZE: usize = 100_000_000;
const WORKERS: usize = 20;

fn main() -> Result<(), Box<dyn Error>> {
    // Channel to send data into the workers
    let (in_tx, in_rx) = bounded::<InMessage>(WORKERS);
    // Channel to send data out from the workers
    let (out_tx, out_rx) = bounded::<OutMessage>(WORKERS);
    std::thread::scope(|scope| {
        for _ in 0..WORKERS {
            scope.spawn(|| worker(&in_rx, &out_tx).unwrap());
        }

        produce_from_csv(&in_tx, &out_rx, IN_PATH, CHUNK_SIZE, WORKERS, OUT_FOLDER)
    })?;

    Ok(())
}
