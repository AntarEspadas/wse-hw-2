use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};

use glob::glob;

use crate::index::data::IndexEntry;
use crate::index::{FinalIndexWriter, IndexReader, IndexWriter, IntermediateIndexWriter};
use crate::page_table::PageTable;

pub mod bm25;
pub mod cli;
pub mod index;
pub mod page_table;

pub fn merge(
    in_dir: &Path,
    out_dir: &Path,
    input_count: usize,
    buffer_size: usize,
) -> Result<(), Box<dyn Error>> {
    if input_count == 0 || buffer_size == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "input count and buffer size must be positive",
        )
        .into());
    }

    let pattern = in_dir.join("index-*.bin");
    let pattern = pattern.to_str().unwrap();
    let mut index_paths: VecDeque<_> = glob(pattern)?
        .filter_map(Result::ok)
        .map(|path| {
            let filename = path.file_name().unwrap().to_str().unwrap();
            let mut lexicon_filename = String::from("lexicon");
            lexicon_filename.push_str(filename.strip_prefix("index").unwrap());
            let lexicon_path = path.parent().unwrap().join(lexicon_filename);
            (path, lexicon_path)
        })
        .collect();

    if index_paths.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no partial indexes found").into());
    }
    if input_count == 1 && index_paths.len() > 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "merging multiple partial indexes requires an input count of at least two",
        )
        .into());
    }

    for i in 0.. {
        let count = input_count.min(index_paths.len());
        let input_paths: Vec<_> = index_paths.drain(..count).collect();

        if index_paths.is_empty() {
            let output_index_path = out_dir.join("index.bin");
            let output_lexicon_path = out_dir.join("lexicon.bin");
            let page_table_path = in_dir.join("page-table.bin");

            let page_table = PageTable::read(&page_table_path)?;

            let index_writer = FinalIndexWriter::open(&output_index_path, page_table, buffer_size)?;

            println!("Pass {i}: Merging last {} indices...", input_paths.len());

            merge_indices(
                &input_paths,
                index_writer,
                &output_lexicon_path,
                buffer_size,
            )?;

            return Ok(());
        }

        let output_index_path = out_dir.join(format!("merged-index-{i}.bin"));
        let output_lexicon_path = out_dir.join(format!("merged-lexicon-{i}.bin"));

        let index_writer = IntermediateIndexWriter::open(&output_index_path, buffer_size)?;

        println!("Pass {}: Merging {input_count} indices...", i);
        merge_indices(
            &input_paths,
            index_writer,
            &output_lexicon_path,
            buffer_size,
        )?;

        index_paths.push_back((output_index_path, output_lexicon_path));
    }
    Ok(())
}

pub fn merge_indices(
    input_paths: &[(PathBuf, PathBuf)],
    mut index_writer: impl IndexWriter,
    output_lexicon_path: &Path,
    buffer_size: usize,
) -> Result<(), Box<dyn Error>> {
    let readers: Result<Vec<_>, Box<dyn Error>> = input_paths
        .iter()
        .map(|x| IndexReader::open(&x.0, &x.1, buffer_size))
        .collect();

    let mut readers = readers?;

    let mut heap: BinaryHeap<HeapElement> = BinaryHeap::with_capacity(input_paths.len());

    for reader in readers.iter_mut() {
        let Some(entry) = reader.next_entry()? else {
            continue;
        };
        let heap_element = HeapElement {
            index_reader: reader,
            entry,
        };

        heap.push(heap_element);
    }

    while let Some(element) = heap.pop() {
        index_writer.write_entry(element.entry)?;

        if let Some(entry) = element.index_reader.next_entry()? {
            heap.push(HeapElement {
                index_reader: element.index_reader,
                entry,
            });
        }
    }

    index_writer.flush()?;

    index_writer.write_lexicon(output_lexicon_path)?;
    Ok(())
}

struct HeapElement<'a> {
    index_reader: &'a mut IndexReader,
    entry: IndexEntry,
}

impl<'a> PartialEq for HeapElement<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.entry.term == other.entry.term
    }
}
impl<'a> Eq for HeapElement<'a> {}

impl<'a> PartialOrd for HeapElement<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for HeapElement<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.entry.term.cmp(&other.entry.term).reverse()
    }
}
