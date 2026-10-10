use std::{
    collections::VecDeque,
    error::Error,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
};

use crate::index::data::{IndexEntry, LexiconEntry};

pub struct IndexWriter {
    writer: BufWriter<File>,
    entries: VecDeque<IndexEntry>,
    lexicon: Vec<LexiconEntry>,
    buffer_size: usize,
}

impl IndexWriter {
    pub fn open(path: &Path, buffer_size: usize) -> Result<Self, Box<dyn Error>> {
        assert!(buffer_size >= 1);

        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;

        let writer = BufWriter::new(file);
        Ok(Self {
            writer,
            entries: VecDeque::with_capacity(buffer_size),
            lexicon: Vec::new(),
            buffer_size,
        })
    }

    pub fn write_entry(&mut self, mut entry: IndexEntry) -> Result<(), Box<dyn Error>> {
        if let Some(last_entry) = self.entries.back_mut()
            && last_entry.term == entry.term
        {
            last_entry.postings.append(&mut entry.postings);
        } else {
            if self.entries.len() >= self.buffer_size {
                self.write_current()?;
                debug_assert_eq!(self.entries.len(), 0);
            }
            self.entries.push_back(entry);
        }
        Ok(())
    }

    pub fn write_current(&mut self) -> Result<(), Box<dyn Error>> {
        let mut offset = self
            .lexicon
            .last()
            .map(|x| x.offset + x.len * size_of::<u32>() * 2)
            .unwrap_or(0);
        while let Some(mut entry) = self.entries.pop_front() {
            let len = entry.postings.len();

            entry.postings.sort_unstable_by_key(|x| x.doc_id);

            let doc_ids: Vec<_> = entry
                .postings
                .iter()
                .flat_map(|x| x.doc_id.to_be_bytes())
                .collect();

            let frequencies: Vec<_> = entry
                .postings
                .iter()
                .flat_map(|x| x.frequency.to_be_bytes())
                .collect();

            self.writer.write_all(&doc_ids)?;
            self.writer.write_all(&frequencies)?;

            self.lexicon.push(LexiconEntry {
                term: entry.term,
                offset,
                len,
            });

            offset += len * size_of::<u32>() * 2;
        }
        Ok(())
    }
}
