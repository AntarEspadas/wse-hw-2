mod lexicon;
mod posting_list;

pub use lexicon::Lexicon;
pub use posting_list::PostingReader;
use std::{
    error::Error,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

use crate::index::lexicon::LexiconEntry;

pub struct Index {
    index_path: PathBuf,
    lexicon: Lexicon,
}

impl Index {
    pub fn new(index_path: PathBuf, lexicon: Lexicon) -> Self {
        Self {
            index_path,
            lexicon,
        }
    }

    pub fn get_postings(&self, term: &str) -> Result<Option<PostingReader>, Box<dyn Error>> {
        let Some(lexicon_entry) = self.lexicon.get(term) else {
            return Ok(None);
        };

        println!(
            "Lexicon entry: len={}, offset={}",
            lexicon_entry.len, lexicon_entry.offset
        );

        let mut file = File::open(&self.index_path)?;

        self.get_postings_from_lexicon_entry(&mut file, *lexicon_entry)
            .map(Some)
    }

    pub fn get_all_postings(&self, terms: &[String]) -> Result<Vec<PostingReader>, Box<dyn Error>> {
        let mut entries: Vec<_> = terms
            .iter()
            .flat_map(|x| self.lexicon.get(x))
            .copied()
            .collect();

        entries.sort_unstable_by_key(|x| x.offset);

        let mut file = File::open(&self.index_path)?;

        entries
            .into_iter()
            .map(|x| self.get_postings_from_lexicon_entry(&mut file, x))
            .collect()
    }

    fn get_postings_from_lexicon_entry(
        &self,
        lexicon_file: &mut File,
        lexicon_entry: LexiconEntry,
    ) -> Result<PostingReader, Box<dyn Error>> {
        let mut buffer: Vec<u8> = Vec::with_capacity(lexicon_entry.len);

        lexicon_file.seek(SeekFrom::Start(lexicon_entry.offset as u64))?;
        lexicon_file
            .by_ref()
            .take(lexicon_entry.len as u64)
            .read_to_end(&mut buffer)?;

        Ok(PostingReader::new(buffer))
    }
}
