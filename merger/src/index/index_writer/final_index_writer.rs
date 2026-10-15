use std::{
    collections::VecDeque,
    error::Error,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
};

use common::compression::simple9::compress;

use crate::{
    bm25,
    index::{
        data::{IndexEntry, LexiconEntry},
        index_writer::IndexWriter,
    },
    page_table::PageTable,
};

pub struct FinalIndexWriter {
    writer: BufWriter<File>,
    entries: VecDeque<IndexEntry>,
    lexicon: Vec<LexiconEntry>,
    page_table: PageTable,
    buffer_size: usize,
    current_offset: usize,
}

impl FinalIndexWriter {
    pub fn open(
        path: &Path,
        page_table: PageTable,
        buffer_size: usize,
    ) -> Result<Self, Box<dyn Error>> {
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
            page_table,
            buffer_size,
            current_offset: 0,
        })
    }

    fn write_current(&mut self) -> Result<(), Box<dyn Error>> {
        while let Some(mut entry) = self.entries.pop_front() {
            entry.postings.sort_unstable_by_key(|x| x.doc_id);

            let mut doc_ids = Vec::with_capacity(entry.postings.len());
            let mut previous = 0u32;

            for posting in entry.postings.iter() {
                let doc_id = posting.doc_id - previous;
                // let doc_id = doc_id.to_be_bytes();
                // doc_ids.extend(&doc_id);
                doc_ids.push(doc_id);
                previous = posting.doc_id
            }

            let mut max_score = 0u32;

            let mut impact_scores = Vec::with_capacity(entry.postings.len());

            for posting in entry.postings.iter() {
                let score = bm25::pre_compute(
                    posting.doc_id,
                    posting.frequency as f64,
                    entry.postings.len() as f64,
                    &self.page_table,
                );
                let score = ((score + 64f32) * 100f32).round() as u32;
                max_score = max_score.max(score);

                impact_scores.push(score);
            }

            let max_score = max_score.to_be_bytes();

            let doc_ids: Vec<_> = compress(&doc_ids).flat_map(u32::to_be_bytes).collect();
            let impact_scores: Vec<_> = compress(&impact_scores)
                .flat_map(u32::to_be_bytes)
                .collect();

            let posting_count = (entry.postings.len() as u32).to_be_bytes();
            let doc_ids_len = (doc_ids.len() as u32).to_be_bytes();
            let scores_len = (impact_scores.len() as u32).to_be_bytes();

            let block_len = posting_count.len()
                + max_score.len()
                + doc_ids_len.len()
                + scores_len.len()
                + doc_ids.len()
                + impact_scores.len();

            self.writer.write_all(&posting_count)?;
            self.writer.write_all(&max_score)?;
            self.writer.write_all(&doc_ids_len)?;
            self.writer.write_all(&scores_len)?;
            self.writer.write_all(&doc_ids)?;
            self.writer.write_all(&impact_scores)?;

            self.lexicon.push(LexiconEntry {
                term: entry.term,
                offset: self.current_offset,
                len: block_len,
            });

            self.current_offset += block_len;
        }
        Ok(())
    }
}

impl IndexWriter for FinalIndexWriter {
    fn write_lexicon(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;

        let mut writer = BufWriter::new(file);

        for entry in self.lexicon.iter() {
            write!(writer, "{}\0", entry.term)?;
            let offset = (entry.offset as u32).to_be_bytes();
            let len = (entry.len as u32).to_be_bytes();
            writer.write_all(&offset)?;
            writer.write_all(&len)?;
        }
        Ok(())
    }

    fn write_entry(&mut self, mut entry: IndexEntry) -> Result<(), Box<dyn Error>> {
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

    fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        self.write_current()?;
        self.writer.flush()?;
        Ok(())
    }
}
