use std::{
    collections::VecDeque,
    error::Error,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
};

pub struct IndexEntry {
    term_id: u32,
    doc_ids: Vec<u32>,
    frequencies: Vec<u32>,
}

pub struct IndexWriter {
    writer: BufWriter<File>,
    entries: VecDeque<IndexEntry>,
}

impl IndexWriter {
    pub fn open(path: &Path) -> Result<Self, Box<dyn Error>> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;

        let writer = BufWriter::new(file);
        Ok(Self {
            writer,
            entries: VecDeque::new(),
        })
    }

    pub fn add_ordered_entry(&mut self, term_id: u32, postings: Vec<(u32, u32)>) {
        if let Some(back) = self.entries.back_mut()
            && back.term_id == term_id
        {
            for (doc_id, freq) in postings {
                back.doc_ids.push(doc_id);
                back.frequencies.push(freq);
            }
        } else {
            let mut doc_ids: Vec<u32> = Vec::with_capacity(postings.len());
            let mut frequencies: Vec<u32> = Vec::with_capacity(postings.len());
            for (doc_id, freq) in postings {
                doc_ids.push(doc_id);
                frequencies.push(freq);
            }

            self.entries.push_back(IndexEntry {
                term_id,
                doc_ids,
                frequencies,
            });
        }
    }

    pub fn write_current(&mut self) -> Result<Vec<(u32, u32)>, Box<dyn Error>> {
        let mut result = Vec::with_capacity(self.entries.len());
        while let Some(entry) = self.entries.pop_front() {
            let doc_ids: Vec<_> = entry
                .doc_ids
                .into_iter()
                .flat_map(|x| x.to_be_bytes())
                .collect();

            let frequencies: Vec<_> = entry
                .frequencies
                .into_iter()
                .flat_map(|x| x.to_be_bytes())
                .collect();

            self.writer.write_all(&doc_ids)?;
            self.writer.write_all(&frequencies)?;

            result.push((entry.term_id, doc_ids.len() as u32));
        }
        Ok(result)
    }
}
