use clap::Parser;
use crossbeam_channel::bounded;
use indexer::data::Lexicon;
use indexer::data::OutMessage;
use indexer::write_lexicon_bin;
use indexer::write_lexicon_plaintext;
use std::error::Error;
use std::println;

use indexer::cli::Args;
use indexer::data::InMessage;
use indexer::produce_from_csv;
use indexer::worker;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    println!("Chunk size: {}", args.chunk_size);

    // Channel to send data into the workers
    let (in_tx, in_rx) = bounded::<InMessage>(args.workers);
    // Channel to send data out from the workers
    let (out_tx, out_rx) = bounded::<OutMessage>(args.workers);
    std::thread::scope(|scope| {
        for _ in 0..args.workers {
            scope.spawn(|| worker(&in_rx, &out_tx).unwrap());
        }

        let lexicon = Lexicon::new();

        produce_from_csv(
            &in_tx,
            &out_rx,
            &args.input,
            args.chunk_size,
            args.workers,
            &args.out,
            &lexicon,
        )
        .unwrap();

        println!("Writing lexicon...");
        let out_path = args.out.join("lexicon.txt");
        write_lexicon_plaintext(&lexicon, &out_path).unwrap();
        let out_path = args.out.join("lexicon.bin");
        write_lexicon_bin(&lexicon, &out_path).unwrap();
    });

    Ok(())
}
