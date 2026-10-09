use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

use merger::index::IndexReader;

fn read_lexicon(path: &Path) -> Result<HashMap<String, u32>, Box<dyn Error>> {
    let file = File::open(path)?;

    let mut reader = BufReader::new(file);

    let mut lexicon: HashMap<String, u32> = HashMap::new();

    println!("Reading lexicon");

    loop {
        let mut term: Vec<u8> = Vec::new();

        let read = reader.read_until(b'\0', &mut term)?;

        if read == 0 {
            break;
        }

        term.truncate(term.len() - 1);

        let term = String::from_utf8(term)?;

        let mut term_id = [0u8; 4];

        reader.read_exact(&mut term_id)?;

        let term_id = u32::from_be_bytes(term_id);

        lexicon.insert(term, term_id);
    }

    Ok(lexicon)
}

fn main() -> Result<(), Box<dyn Error>> {
    let lengths_path = Path::new("F:\\output\\lengths-0.bin");
    let index_path = Path::new("F:\\output\\index-0.bin");

    let mut index_reader = IndexReader::open(index_path, lengths_path)?;

    let entries = index_reader.read_entries(10)?;

    for entry in entries {
        println!("term_id: {}", entry.term_id);
    }

    let entries = index_reader.read_entries(10)?;

    for entry in entries {
        println!("term_id: {}", entry.term_id);
    }

    // let path = Path::new("F:\\output\\lexicon.bin");
    // let lexicon = read_lexicon(path)?;

    // let file = fs::File::open("F:\\output\\index-0.bin")?;

    // let mut reader = BufReader::new(file);
    // for length in lengths.into_iter() {
    //     let mut term_id = [9u8; 4];

    //     reader.read_exact(&mut term_id)?;

    //     let term_id = u32::from_be_bytes(term_id);

    //     let size = length as usize * size_of::<(u32, u32)>();
    //     let mut postings_buf: Vec<u8> = Vec::with_capacity(size);

    //     reader
    //         .by_ref()
    //         .take(size as u64)
    //         .read_to_end(&mut postings_buf)?;

    //     let postings: Vec<_> = postings_buf
    //         .as_chunks::<8>()
    //         .0
    //         .iter()
    //         .map(|buf| {
    //             let doc_id = u32::from_be_bytes(buf[..4].try_into().unwrap());
    //             let count = u32::from_be_bytes(buf[4..].try_into().unwrap());
    //             (doc_id, count)
    //         })
    //         .collect();

    //     println!("{term_id} {length}");
    //     let postings_10: Vec<_> = postings.iter().take(10).collect();
    //     println!("{:?}", postings_10);
    // }

    Ok(())
}
