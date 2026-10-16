use std::{
    collections::VecDeque,
    error::Error,
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
};

use common::{
    compression::simple9::compress,
    index::{BlockMetadata, PostingMetadata},
};

use crate::{
    bm25,
    index::{
        data::{IndexEntry, LexiconEntry, Posting},
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
            let block_size = 128;

            entry.postings.sort_unstable_by_key(|x| x.doc_id);

            let (metadata, doc_ids, impact_score) =
                self.compress_postings(&entry.postings, block_size);

            let metadata_bytes = metadata.to_bytes();
            self.writer.write_all(&metadata_bytes)?;
            self.writer.write_all(&doc_ids)?;
            self.writer.write_all(&impact_score)?;

            let len = metadata_bytes.len() + doc_ids.len() + impact_score.len();

            // let posting_count = (entry.postings.len() as u32).to_be_bytes();
            // let doc_ids_len = (doc_ids.len() as u32).to_be_bytes();
            // let scores_len = (impact_scores.len() as u32).to_be_bytes();

            // let block_len = posting_count.len()
            //     + max_score.len()
            //     + doc_ids_len.len()
            //     + scores_len.len()
            //     + doc_ids.len()
            //     + impact_scores.len();

            // self.writer.write_all(&posting_count)?;
            // self.writer.write_all(&max_score)?;
            // self.writer.write_all(&doc_ids_len)?;
            // self.writer.write_all(&scores_len)?;
            // self.writer.write_all(&doc_ids)?;
            // self.writer.write_all(&impact_scores)?;

            self.lexicon.push(LexiconEntry {
                term: entry.term,
                offset: self.current_offset,
                len,
            });

            self.current_offset += len;
        }
        Ok(())
    }

    fn compress_doc_ids(&self, postings: &[Posting]) -> (Vec<u8>, u32) {
        let mut doc_ids = Vec::with_capacity(postings.len());
        let mut last_doc_id = 0u32;

        for posting in postings.iter() {
            let doc_id = posting.doc_id - last_doc_id;
            doc_ids.push(doc_id);
            last_doc_id = posting.doc_id
        }

        let doc_ids = compress(&doc_ids).flat_map(u32::to_be_bytes).collect();

        (doc_ids, last_doc_id)
    }

    fn compress_impact_scores(&self, postings: &[Posting]) -> (Vec<u8>, u32) {
        let mut max_score = 0u32;

        let mut impact_scores = Vec::with_capacity(postings.len());

        for posting in postings.iter() {
            let score = bm25::pre_compute(
                posting.doc_id,
                posting.frequency as f64,
                postings.len() as f64,
                &self.page_table,
            );
            let score = ((score + 64f32) * 100f32).round() as u32;
            max_score = max_score.max(score);

            impact_scores.push(score);
        }

        let impact_scores: Vec<_> = compress(&impact_scores)
            .flat_map(u32::to_be_bytes)
            .collect();

        (impact_scores, max_score)
    }

    fn compress_postings(
        &self,
        postings: &[Posting],
        block_size: usize,
    ) -> (BlockMetadata, Vec<u8>, Vec<u8>) {
        let mut compressed_doc_ids: Vec<u8> = Vec::new();
        let mut posting_metadata: Vec<PostingMetadata> = Vec::new();
        let mut overall_max_score = 0u32;
        let mut compressed_scores: Vec<u8> = Vec::new();

        for postings in postings.chunks(block_size) {
            let (mut doc_ids, last_doc_id) = self.compress_doc_ids(postings);
            let (mut scores, max_score) = self.compress_impact_scores(postings);

            posting_metadata.push(PostingMetadata {
                last_doc_id,
                doc_ids_len: doc_ids.len() as u32,
                scores_len: scores.len() as u32,
            });

            overall_max_score = overall_max_score.max(max_score);
            compressed_scores.append(&mut scores);
            compressed_doc_ids.append(&mut doc_ids);
        }

        let block_metadata = BlockMetadata {
            max_score: overall_max_score,
            posting_count: postings.len() as u32,
            posting_metadata,
        };

        (block_metadata, compressed_doc_ids, compressed_scores)
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
