use std::{error::Error, path::Path};

use merger::index::IndexReader;

const BUFFER_SIZE: usize = 100;

fn main() -> Result<(), Box<dyn Error>> {
    let lexicon_path = Path::new("F:\\output\\lexicon-0.bin");
    let index_path = Path::new("F:\\output\\index-0.bin");

    let mut index_reader = IndexReader::open(index_path, lexicon_path, BUFFER_SIZE)?;

    for _ in 0..10 {
        let Some(entry) = index_reader.next_entry()? else {
            break;
        };
        println!("term: {}", entry.term);
        println!("Postings: {}", entry.postings.len());
    }

    for _ in 0..10 {
        let Some(entry) = index_reader.next_entry()? else {
            break;
        };
        println!("term: {}", entry.term);
        println!("Postings: {}", entry.postings.len());
    }

    Ok(())
}
