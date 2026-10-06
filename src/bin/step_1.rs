use crossbeam_channel::bounded;
use std::error::Error;
use std::path::PathBuf;
use wse_hw_2::data::Lexicon;
use wse_hw_2::data::OutMessage;
use wse_hw_2::write_lexicon_bin;
use wse_hw_2::write_lexicon_plaintext;

use wse_hw_2::data::InMessage;
use wse_hw_2::produce_from_csv;
use wse_hw_2::worker;

const OUT_FOLDER: &str = "output";
const IN_PATH: &str = "./data/collection.tsv";
const CHUNK_SIZE: usize = 250_000_000;
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

        let lexicon = Lexicon::new();

        produce_from_csv(
            &in_tx, &out_rx, IN_PATH, CHUNK_SIZE, WORKERS, OUT_FOLDER, &lexicon,
        )
        .unwrap();

        println!("Writing lexicon...");
        let out_path: PathBuf = [OUT_FOLDER, "lexicon.txt"].iter().collect();
        write_lexicon_plaintext(&lexicon, &out_path).unwrap();
        let out_path: PathBuf = [OUT_FOLDER, "lexicon"].iter().collect();
        write_lexicon_bin(&lexicon, &out_path).unwrap();
    });

    Ok(())
}
