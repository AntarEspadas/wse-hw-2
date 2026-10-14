mod lexicon;
mod posting_list;

pub use lexicon::Lexicon;
pub use posting_list::PostingReader;
use std::{
    error::Error,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use crate::index::lexicon::LexiconEntry;

pub struct Index {
    file: File,
    lexicon: Lexicon,
}

impl Index {
    pub fn open(path: &Path, lexicon: Lexicon) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        Ok(Self { file, lexicon })
    }

    pub fn get_postings(&mut self, term: &str) -> Result<Option<PostingReader>, Box<dyn Error>> {
        let Some(lexicon_entry) = self.lexicon.get(term) else {
            return Ok(None);
        };

        println!(
            "Lexicon entry: len={}, offset={}",
            lexicon_entry.len, lexicon_entry.offset
        );

        self.get_postings_from_lexicon_entry(*lexicon_entry)
            .map(Some)
    }

    pub fn get_all_postings(
        &mut self,
        terms: &[String],
    ) -> Result<Vec<PostingReader>, Box<dyn Error>> {
        let mut entries: Vec<_> = terms
            .iter()
            .flat_map(|x| self.lexicon.get(x))
            .copied()
            .collect();

        entries.sort_unstable_by_key(|x| x.offset);

        entries
            .into_iter()
            .map(|x| self.get_postings_from_lexicon_entry(x))
            .collect()
    }

    fn get_postings_from_lexicon_entry(
        &mut self,
        lexicon_entry: LexiconEntry,
    ) -> Result<PostingReader, Box<dyn Error>> {
        const U32_SIZE: usize = size_of::<u32>();

        let len = lexicon_entry.len * U32_SIZE * 2 + U32_SIZE;
        let mut buffer: Vec<u8> = Vec::with_capacity(len);

        self.file
            .seek(SeekFrom::Start(lexicon_entry.offset as u64))?;
        self.file
            .by_ref()
            .take(len as u64)
            .read_to_end(&mut buffer)?;

        Ok(PostingReader::new(buffer))
    }
}
