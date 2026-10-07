use clap::Parser;
use crossbeam_channel::bounded;
use indexer::data::Lexicon;
use indexer::data::OutMessage;
use indexer::write_lexicon_bin;
use indexer::write_lexicon_plaintext;
use std::error::Error;
use std::println;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use indexer::cli::Args;
use indexer::data::InMessage;
use indexer::produce_from_csv;
use indexer::worker;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    // Channel to send data into the workers
    let (in_tx, in_rx) = bounded::<InMessage>(args.workers);
    // Channel to send data out from the workers
    let (out_tx, out_rx) = bounded::<OutMessage>(args.workers);
    std::thread::scope(|scope| {
        for i in 0..args.workers {
            let in_rx = &in_rx;
            let out_tx = &out_tx;
            scope.spawn(move || worker(in_rx, out_tx, i).unwrap());
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

        println!(
            "Writing lexicon with {} terms (capacity: {})...",
            lexicon.len(),
            lexicon.capacity()
        );
        let out_path = args.out.join("lexicon.txt");
        write_lexicon_plaintext(&lexicon, &out_path).unwrap();
        let out_path = args.out.join("lexicon.bin");
        write_lexicon_bin(&lexicon, &out_path).unwrap();
    });

    let finish = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    let elapsed = finish - start;

    println!("Took {:?}", elapsed);

    Ok(())
}
