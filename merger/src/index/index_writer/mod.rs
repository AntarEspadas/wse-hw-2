mod final_index_writer;
mod intermediate_index_writer;

use std::{error::Error, path::Path};

pub use final_index_writer::FinalIndexWriter;
pub use intermediate_index_writer::IntermediateIndexWriter;

use crate::index::data::IndexEntry;

pub trait IndexWriter {
    fn write_entry(&mut self, entry: IndexEntry) -> Result<(), Box<dyn Error>>;

    fn flush(&mut self) -> Result<(), Box<dyn Error>>;

    fn write_lexicon(&self, path: &Path) -> Result<(), Box<dyn Error>>;
}
