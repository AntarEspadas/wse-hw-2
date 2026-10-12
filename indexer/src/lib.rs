use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::{iter, println, thread};

use crate::data::{Index, IndexElement, TermCounter};
use crate::page_table::{Document, PageTable};

pub mod cli;
pub mod data;
pub mod page_table;

fn term(token: &str) -> String {
    let mut t = token.to_lowercase();

    // Remove punctuation
    t.retain(|c| c.is_alphanumeric());

    t
}

fn write_index_plaintext<'a, I>(index: I, out_path: &Path) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = &'a IndexElement>,
{
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;
    let mut writer = BufWriter::new(file);

    for (term, postings) in index {
        write!(writer, "{term};{};", postings.len())?;
        for (doc_id, _) in postings.iter() {
            write!(writer, " {}", doc_id)?;
        }
        for (_, count) in postings.iter() {
            write!(writer, " {}", count)?;
        }
        writeln!(writer)?;
    }

    writer.flush()?;
    Ok(())
}

fn write_index_bin<'a, I>(index: I, out_path: &Path) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = &'a IndexElement>,
{
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;
    let mut writer = BufWriter::new(file);

    for (_, postings) in index {
        for (doc_id, _) in postings {
            writer.write_all(&doc_id.to_be_bytes())?;
        }
        for (_, count) in postings {
            writer.write_all(&count.to_be_bytes())?;
        }
    }
    writer.flush()?;
    Ok(())
}

fn write_lexicon_plaintext<'a, I>(index: I, out_path: &Path) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = &'a IndexElement>,
{
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;

    let mut writer = BufWriter::new(file);

    for (term, postings) in index {
        writeln!(writer, "{term};{}", postings.len())?;
    }

    writer.flush()?;
    Ok(())
}

fn write_lexicon_bin<'a, I>(index: I, out_path: &Path) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = &'a IndexElement>,
{
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;

    let mut writer = BufWriter::new(file);

    let mut offset = 0u32;
    for (term, postings) in index {
        write!(writer, "{term}\0")?;
        let offset_bytes = offset.to_be_bytes();
        let length = (postings.len() as u32).to_be_bytes();
        writer.write_all(&offset_bytes)?;
        writer.write_all(&length)?;

        offset += postings.len() as u32 * size_of::<u32>() as u32 * 2;
    }

    writer.flush()?;
    Ok(())
}

fn process_buffer(
    buffer: &[u8],
    page_table: &mut Option<PageTable>,
    index: &Index,
    // lexicon: &ReadOnlyView<String, usize>,
) -> Result<(), Box<dyn Error>> {
    let mut term_counter = TermCounter::new();
    for line in str::from_utf8(buffer)?.lines() {
        let mut iterator = line.split('\t');
        let doc_id = iterator.next().unwrap();
        let doc_id: u32 = doc_id.parse()?;

        let content = iterator.next().unwrap();

        let page_table = page_table.get_or_insert_with(|| PageTable::new(doc_id));

        page_table.add(Document {
            id: doc_id,
            len: content.len(),
        });

        for token in content.split_whitespace() {
            let t = term(token);
            if !t.is_empty() {
                term_counter.count(t);
            }
        }

        for (term, count) in term_counter.drain() {
            index
                .entry(term)
                .and_modify(|x| x.push((doc_id, count)))
                .or_insert_with(|| vec![(doc_id, count)]);
        }
    }
    Ok(())
}

pub fn generate_index(
    csv_path: &Path,
    chunk_size: usize,
    workers: usize,
    out_folder: &Path,
    write_txt: bool,
    write_bin: bool,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(csv_path)?;

    println!("chunk_size: {chunk_size}, workers: {workers}");
    let buffer_size = chunk_size / workers;

    let mut reader = BufReader::new(file);

    let mut buffers: Vec<_> = iter::repeat_with(|| Vec::with_capacity(buffer_size))
        .take(workers)
        .collect();

    let page_table_path = out_folder.join("page-table.bin");
    let page_table_file = OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(page_table_path)?;
    let mut page_table_writer = BufWriter::new(page_table_file);

    for i in 0.. {
        let mut read = 0;

        let index = Index::new();
        let mut page_tables: Vec<Option<PageTable>> =
            iter::repeat_with(|| None).take(workers).collect();

        println!("Indexing...");
        thread::scope(|scope| -> Result<(), Box<dyn Error>> {
            for (buffer, page_table) in buffers.iter_mut().zip(page_tables.iter_mut()) {
                buffer.clear();
                read += read_lines_into_buffer(&mut reader, buffer, buffer_size)?;
                let index = &index;
                scope.spawn(move || process_buffer(buffer, page_table, index).unwrap());
            }
            Ok(())
        })?;

        if read == 0 {
            break;
        }

        println!(
            "Iteration {i}: indexed {} terms (index capacity: {})",
            index.len(),
            index.capacity()
        );

        let mut index: Vec<_> = index
            .into_iter()
            .map(|mut x| {
                x.1.sort_unstable_by_key(|x| x.0);
                x
            })
            .collect();

        index.sort_unstable_by(|x, y| x.0.cmp(&y.0));

        let mut page_tables: Vec<_> = page_tables.into_iter().flatten().collect();
        page_tables.sort_unstable_by_key(PageTable::first_doc_id);

        if write_txt {
            let filename = format!("index-{i}.txt");
            let out_path = out_folder.join(&filename);

            println!("Writing index to {:?}", out_path);

            write_index_plaintext(index.iter(), &out_path)?;

            let filename = format!("lexicon-{i}.txt");
            let out_path = out_folder.join(&filename);

            println!("Writing lexicon to {:?}", out_path);

            write_lexicon_plaintext(index.iter(), &out_path)?;
        }

        if write_bin {
            let filename = format!("index-{i}.bin");
            let out_path = out_folder.join(&filename);

            println!("Writing index to {:?}", out_path);

            write_index_bin(index.iter(), &out_path)?;

            let filename = format!("lexicon-{i}.bin");
            let out_path = out_folder.join(&filename);

            println!("Writing lexicon to {:?}", out_path);

            write_lexicon_bin(index.iter(), &out_path)?;

            let out_path = out_folder.join("page-table.bin");

            println!("Writing page table to {:?}", out_path);
            for page_table in page_tables {
                page_table.write(&mut page_table_writer)?;
            }
        }
    }

    Ok(())
}

fn read_lines_into_buffer(
    reader: &mut BufReader<File>,
    buffer: &mut Vec<u8>,
    bytes: usize,
) -> Result<usize, Box<dyn Error>> {
    let mut size = reader.by_ref().take(bytes as u64).read_to_end(buffer)?;

    size += reader.read_until(b'\n', buffer)?;
    Ok(size)
}
