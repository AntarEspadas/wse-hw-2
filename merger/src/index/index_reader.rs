use std::{
    collections::VecDeque,
    error::Error,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

pub struct IndexEntry {
    pub term_id: u32,
    pub postings: Vec<(u32, u32)>,
}

pub struct IndexReader {
    lengths: Vec<u32>,
    reader: BufReader<File>,
    i: usize,
}

impl IndexReader {
    pub fn open(index_path: &Path, lengths_path: &Path) -> Result<Self, Box<dyn Error>> {
        let lengths = IndexReader::read_lengths(lengths_path)?;

        let index_file = File::open(index_path)?;

        let reader = BufReader::new(index_file);

        Ok(Self {
            lengths,
            reader,
            i: 0,
        })
    }

    fn read_lengths(path: &Path) -> Result<Vec<u32>, Box<dyn Error>> {
        let mut file = File::open(path)?;

        let mut buffer: Vec<u8> = Vec::new();

        file.read_to_end(&mut buffer)?;

        Ok(buffer
            .as_chunks::<4>()
            .0
            .iter()
            .map(|x| u32::from_be_bytes(*x))
            .collect())
    }

    pub fn read_entries(&mut self, n: usize) -> Result<VecDeque<IndexEntry>, Box<dyn Error>> {
        let mut result = VecDeque::with_capacity(n);
        for i in self.lengths.iter().skip(self.i).take(n) {
            let mut term_id = [9u8; 4];

            self.reader.read_exact(&mut term_id)?;

            let term_id = u32::from_be_bytes(term_id);

            let size = *i as usize * size_of::<(u32, u32)>();
            let mut postings_buf: Vec<u8> = Vec::with_capacity(size);

            self.reader
                .by_ref()
                .take(size as u64)
                .read_to_end(&mut postings_buf)?;

            let postings: Vec<_> = postings_buf
                .as_chunks::<8>()
                .0
                .iter()
                .map(|buf| {
                    let doc_id = u32::from_be_bytes(buf[..4].try_into().unwrap());
                    let count = u32::from_be_bytes(buf[4..].try_into().unwrap());
                    (doc_id, count)
                })
                .collect();

            result.push_back(IndexEntry { term_id, postings });
        }

        self.i += n;

        Ok(result)
    }
}
