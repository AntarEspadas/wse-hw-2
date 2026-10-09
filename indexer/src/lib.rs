use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::{iter, println, thread};

use dashmap::ReadOnlyView;
use parking_lot::Mutex;

use crate::data::{Index, IndexElement, Lexicon, TermCounter};

pub mod cli;
pub mod data;

pub fn term(token: &str) -> String {
    let mut t = token.to_lowercase();

    // Remove punctuation
    t.retain(|c| c.is_alphanumeric());

    t
}

pub fn write_index_plaintext<'a, I>(index: I, out_path: &Path) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = &'a mut IndexElement>,
{
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;
    let mut writer = BufWriter::new(file);

    for (term_id, postings) in index.flat_map(|x| x.get_mut()).enumerate() {
        write!(writer, "{term_id};{};", postings.len())?;
        for (doc_id, count) in postings {
            write!(writer, " {} {}", doc_id, count)?;
        }
        writeln!(writer)?;
    }

    writer.flush()?;
    Ok(())
}

pub fn write_index_bin<'a, I>(index: I, out_path: &Path) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = &'a mut IndexElement>,
{
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;
    let mut writer = BufWriter::new(file);

    for (term_id, postings) in index.flat_map(|x| x.get_mut()).enumerate() {
        let term = (term_id as u32).to_be_bytes();
        writer.write_all(&term)?;

        let len = postings.len() as u32;
        writer.write_all(&len.to_be_bytes())?;

        for (doc_id, count) in postings {
            writer.write_all(&doc_id.to_be_bytes())?;
            writer.write_all(&count.to_be_bytes())?;
        }
    }
    writer.flush()?;
    Ok(())
}

pub fn write_lexicon_plaintext<'a, I>(lexicon: I, out_path: &Path) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = (&'a String, &'a usize)>,
{
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;

    let mut writer = BufWriter::new(file);

    for (term, term_id) in lexicon {
        writeln!(writer, "{term};{term_id}")?;
    }

    writer.flush()?;
    Ok(())
}

pub fn write_lexicon_bin<'a, I>(lexicon: I, out_path: &Path) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = (&'a String, &'a usize)>,
{
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;

    let mut writer = BufWriter::new(file);

    for (term, term_id) in lexicon {
        write!(writer, "{term}\0")?;
        let term_id = (*term_id as u32).to_be_bytes();
        writer.write_all(&term_id)?;
    }

    writer.flush()?;
    Ok(())
}

pub fn process_buffer(
    buffer: &[u8],
    index: &Index,
    lexicon: &ReadOnlyView<String, usize>,
) -> Result<(), Box<dyn Error>> {
    let mut term_counter = TermCounter::new();
    for line in str::from_utf8(buffer)?.lines() {
        let mut iterator = line.split('\t');
        let doc_id = iterator.next().unwrap();
        let doc_id: u32 = doc_id.parse()?;

        let content = iterator.next().unwrap();

        for token in content.split_whitespace() {
            let t = term(token);
            if !t.is_empty() {
                term_counter.count(t);
            }
        }

        for (term, count) in term_counter.drain() {
            let term_id = *lexicon.get(&term).unwrap();

            let mut postings = index[term_id].lock();

            postings.get_or_insert_with(Vec::new).push((doc_id, count));
        }
    }
    Ok(())
}

fn populate_lexicon(buffer: &[u8], lexicon: &Lexicon) -> Result<(), Box<dyn Error>> {
    for line in str::from_utf8(buffer)?.lines() {
        let mut iterator = line.split('\t');

        let content = iterator.nth(1).unwrap();

        for token in content.split_whitespace() {
            let t = term(token);
            lexicon.add(t);
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
    let mut lexicon = Lexicon::new();

    println!("chunk_size: {chunk_size}, workers: {workers}");
    let buffer_size = chunk_size / workers;

    let mut reader = BufReader::new(file);

    let mut buffers: Vec<_> = iter::repeat_with(|| Vec::with_capacity(buffer_size))
        .take(workers)
        .collect();

    for i in 0.. {
        let mut read = 0;

        println!("Reading {chunk_size} bytes of data and populating lexicon...");
        thread::scope(|scope| {
            for buffer in buffers.iter_mut() {
                buffer.clear();
                read += read_lines_into_buffer(&mut reader, buffer, buffer_size).unwrap();

                let lexicon = &lexicon;
                scope.spawn(move || {
                    populate_lexicon(buffer, lexicon).unwrap();
                });
            }
        });
        println!("Read {chunk_size} bytes of data");
        println!("Lexicon size: {}", lexicon.len());

        let readonly_lexicon = lexicon.into_readonly();
        let index: Index = iter::repeat_with(|| Mutex::new(None))
            .take(readonly_lexicon.len())
            .collect();

        println!("Indexing...");
        thread::scope(|scope| {
            for buffer in buffers.iter_mut() {
                let index = &index;
                let readonly_lexicon = &readonly_lexicon;
                scope.spawn(move || process_buffer(buffer, index, readonly_lexicon).unwrap());
            }
        });
        lexicon = Lexicon::from_readonly(readonly_lexicon);

        if read == 0 {
            break;
        }

        println!(
            "Iteration {i}: indexed {} terms (index capacity: {})",
            index.len(),
            index.capacity()
        );

        let mut index = index;

        for postings in index.iter_mut() {
            let postings = postings.get_mut();
            if let Some(postings) = postings {
                postings.sort_unstable_by_key(|x| x.0);
            }
        }

        if write_txt {
            let filename = format!("index-{i}.txt");
            let out_path = out_folder.join(&filename);

            println!("Writing index of size {} to {:?}", index.len(), out_path);

            write_index_plaintext(index.iter_mut(), &out_path)?;
        }

        if write_bin {
            let filename = format!("index-{i}.bin");
            let out_path = out_folder.join(&filename);

            println!("Writing index of size {} to {:?}", index.len(), out_path);

            write_index_bin(index.iter_mut(), &out_path)?;
        }
    }

    println!("Writing lexicon...");

    let lexicon = lexicon.into_readonly();

    if write_txt {
        let out_path = out_folder.join("lexicon.bin");
        write_lexicon_bin(lexicon.iter(), &out_path).unwrap();
    }
    if write_bin {
        let out_path = out_folder.join("lexicon.txt");
        write_lexicon_plaintext(lexicon.iter(), &out_path).unwrap();
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
