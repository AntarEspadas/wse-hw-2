use std::{
    error::Error,
    fs,
    io::{BufRead, BufReader, Read},
};

fn main() -> Result<(), Box<dyn Error>> {
    let file = fs::File::open("output/index-0")?;

    let mut reader = BufReader::new(file);

    loop {
        let mut term: Vec<u8> = Vec::new();

        let read = reader.read_until(b'\0', &mut term)?;

        if read == 0 {
            break;
        }

        term.truncate(term.len() - 1);

        let term = str::from_utf8(&term)?;

        let mut len_buf = [0u8; 4];

        reader.read_exact(&mut len_buf)?;

        let len = u32::from_be_bytes(len_buf);

        let mut postings_buf: Vec<u8> = Vec::with_capacity((len * 8) as usize);

        reader
            .by_ref()
            .take((len * 8) as u64)
            .read_to_end(&mut postings_buf)?;

        let (chunks, _) = postings_buf.as_chunks::<8>();
        let postings: Vec<_> = chunks
            .iter()
            .map(|buf| {
                let doc_id = u32::from_be_bytes(buf[..4].try_into().unwrap());
                let count = u32::from_be_bytes(buf[4..].try_into().unwrap());
                (doc_id, count)
            })
            .collect();

        println!("{term} {len}");
        let postings_10: Vec<_> = postings.iter().take(10).collect();
        println!("{:?}", postings_10);
    }

    Ok(())
}
