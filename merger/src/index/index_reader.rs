use std::{
    collections::VecDeque,
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

use crate::index::data::{IndexEntry, LexiconEntry, Posting};

pub struct IndexReader {
    lexicon: VecDeque<LexiconEntry>,
    entries: VecDeque<IndexEntry>,
    reader: BufReader<File>,
    buffer_size: usize,
}

impl IndexReader {
    pub fn open(
        index_path: &Path,
        lexicon_path: &Path,
        buffer_size: usize,
    ) -> Result<Self, Box<dyn Error>> {
        assert!(buffer_size >= 1);
        let lexicon = IndexReader::read_lexicon(lexicon_path)?;

        let index_file = File::open(index_path)?;

        let reader = BufReader::new(index_file);

        Ok(Self {
            lexicon,
            entries: VecDeque::with_capacity(buffer_size),
            reader,
            buffer_size,
        })
    }

    fn read_lexicon(path: &Path) -> Result<VecDeque<LexiconEntry>, Box<dyn Error>> {
        let file = File::open(path)?;

        let mut reader = BufReader::new(file);

        let mut result = VecDeque::new();

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

            result.push_back(LexiconEntry {
                term,
                offset: u32::from_be_bytes(offset) as usize,
                len: u32::from_be_bytes(length) as usize,
            });
        }

        Ok(result)
    }

    pub fn next_entry(&mut self) -> Result<Option<IndexEntry>, Box<dyn Error>> {
        let entry = self.entries.pop_front();
        if entry.is_some() {
            return Ok(entry);
        }

        for _ in 0..self.buffer_size {
            let Some(entry) = self.lexicon.pop_front() else {
                return Ok(None);
            };
            let length = entry.len * size_of::<u32>();
            let mut doc_ids: Vec<u8> = Vec::with_capacity(length);

            self.reader
                .by_ref()
                .take(length as u64)
                .read_to_end(&mut doc_ids)?;

            let doc_ids: Vec<_> = doc_ids
                .as_chunks::<4>()
                .0
                .iter()
                .map(|buf| u32::from_be_bytes(buf.as_slice().try_into().unwrap()))
                .collect();

            let mut frequencies: Vec<u8> = Vec::with_capacity(length);
            self.reader
                .by_ref()
                .take(length as u64)
                .read_to_end(&mut frequencies)?;

            let frequencies: Vec<_> = frequencies
                .as_chunks::<4>()
                .0
                .iter()
                .map(|buf| u32::from_be_bytes(buf.as_slice().try_into().unwrap()))
                .collect();

            let postings: Vec<_> = doc_ids
                .into_iter()
                .zip(frequencies)
                .map(|(doc_id, frequency)| Posting { doc_id, frequency })
                .collect();

            self.entries.push_back(IndexEntry {
                term: entry.term,
                postings,
            });
        }

        Ok(self.entries.pop_front())
    }
}
