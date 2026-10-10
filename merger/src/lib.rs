use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::error::Error;
use std::path::{Path, PathBuf};

use glob::glob;

use crate::index::data::IndexEntry;
use crate::index::{IndexReader, IndexWriter};

pub mod index;

pub fn merge(
    work_folder: &Path,
    chunks: usize,
    buffer_size: usize,
) -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let pattern = work_folder.join("index-*.bin");
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

    for i in 0.. {
        if index_paths.len() <= 1 {
            break;
        }
        let input_paths: Vec<_> = index_paths.drain(..chunks).collect();

        let output_index_path = work_folder.join(format!("merged-index-{i}.bin"));
        let output_lexicon_path = work_folder.join(format!("merged-lexicon-{i}.bin"));

        merge_indices(
            &input_paths,
            &output_index_path,
            &output_lexicon_path,
            buffer_size,
        )?;

        index_paths.push_back((output_index_path, output_lexicon_path));
    }

    Ok(index_paths.pop_front().unwrap())
}

pub fn merge_indices(
    input_paths: &[(PathBuf, PathBuf)],
    output_index_path: &Path,
    output_lexicon_path: &Path,
    buffer_size: usize,
) -> Result<(), Box<dyn Error>> {
    println!("Creating writer...");
    let mut writer = IndexWriter::open(output_index_path, buffer_size)?;

    println!("Creating readers...");
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
        writer.write_entry(element.entry)?;

        if let Some(entry) = element.index_reader.next_entry()? {
            heap.push(HeapElement {
                index_reader: element.index_reader,
                entry,
            });
        }
    }

    writer.flush()?;

    writer.write_lexicon(output_lexicon_path)?;
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
