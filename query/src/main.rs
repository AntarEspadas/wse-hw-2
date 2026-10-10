use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
};
pub struct Posting {
    pub doc_id: u32,
    pub frequency: u32,
}
pub struct LexiconEntry {
    pub term: String,
    pub offset: usize,
    pub len: usize,
}

fn read_lexicon(path: &Path) -> Result<Vec<LexiconEntry>, Box<dyn Error>> {
    let file = File::open(path)?;

    let mut reader = BufReader::new(file);

    let mut result = Vec::new();

    loop {
        let mut term: Vec<u8> = Vec::new();
        let read = reader.read_until(b'\0', &mut term)?;

        if read == 0 {
            break;
        }

        // Remove null byte from end
        term.truncate(term.len() - 1);

        let term = String::from_utf8(term)?;

        let mut offset = [0u8; 4];
        let mut length = [0u8; 4];

        reader.read_exact(&mut offset)?;
        reader.read_exact(&mut length)?;

        result.push(LexiconEntry {
            term,
            offset: u32::from_be_bytes(offset) as usize,
            len: u32::from_be_bytes(length) as usize,
        });
    }

    Ok(result)
}

fn get_postings(
    index_path: &Path,
    offset: usize,
    len: usize,
) -> Result<Vec<Posting>, Box<dyn Error>> {
    let file = File::open(index_path)?;
    let mut reader = BufReader::new(file);

    let size = len * size_of::<u32>();

    let mut doc_ids = Vec::with_capacity(size);
    let mut frequencies = Vec::with_capacity(size);

    reader.seek(SeekFrom::Start(offset as u64))?;

    reader
        .by_ref()
        .take(size as u64)
        .read_to_end(&mut doc_ids)?;
    reader
        .by_ref()
        .take(size as u64)
        .read_to_end(&mut frequencies)?;

    let doc_ids: Vec<_> = doc_ids
        .as_chunks::<4>()
        .0
        .iter()
        .map(|buf| u32::from_be_bytes(buf.as_slice().try_into().unwrap()))
        .collect();

    let frequencies: Vec<_> = frequencies
        .as_chunks::<4>()
        .0
        .iter()
        .map(|buf| u32::from_be_bytes(buf.as_slice().try_into().unwrap()))
        .collect();

    Ok(doc_ids
        .into_iter()
        .zip(frequencies)
        .map(|(doc_id, frequency)| Posting { doc_id, frequency })
        .collect())
}

fn main() -> Result<(), Box<dyn Error>> {
    let lexicon_path = Path::new("F:\\output\\merged-lexicon-29.bin");
    let index_path = Path::new("F:\\output\\merged-index-29.bin");
    let lexicon = read_lexicon(lexicon_path)?;
    let entry = lexicon.iter().find(|x| x.term == "manhattan").unwrap();
    let mut postings = get_postings(index_path, entry.offset, entry.len)?;

    postings.sort_unstable_by_key(|x| u32::MAX - x.frequency);

    println!("Found {} postings", postings.len());

    let best_match = postings.first().unwrap();

    println!(
        "Best match: {} (Frequency: {})",
        best_match.doc_id, best_match.frequency
    );

    // for posting in postings {
    //     println!("doc_id: {}", posting.doc_id);
    //     println!("frequency: {}", posting.frequency)
    // }
    Ok(())
}
