use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::{iter, println, thread};

use crate::data::{Index, IndexElement, TermCounter};

pub mod cli;
pub mod data;

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
    index: &Index,
    // lexicon: &ReadOnlyView<String, usize>,
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

    for i in 0.. {
        let mut read = 0;

        let index = Index::new();

        println!("Indexing...");
        thread::scope(|scope| {
            for buffer in buffers.iter_mut() {
                buffer.clear();
                read += read_lines_into_buffer(&mut reader, buffer, buffer_size).unwrap();
                let index = &index;
                scope.spawn(move || process_buffer(buffer, index).unwrap());
            }
        });

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

        if write_txt {
            let filename = format!("index-{i}.txt");
            let out_path = out_folder.join(&filename);

            println!("Writing index to {:?}", out_path);

            write_index_plaintext(index.iter(), &out_path)?;

            let filename = format!("lexicon-{i}.txt");
            let out_path = out_folder.join(&filename);

            println!("Writing lexicon to {:?}", out_path);

            write_lexicon_plaintext(index.iter(), &out_path).unwrap();
        }

        if write_bin {
            let filename = format!("index-{i}.bin");
            let out_path = out_folder.join(&filename);

            println!("Writing index to {:?}", out_path);

            write_index_bin(index.iter(), &out_path)?;

            let filename = format!("lexicon-{i}.bin");
            let out_path = out_folder.join(&filename);

            println!("Writing lexicon to {:?}", out_path);

            write_lexicon_bin(index.iter(), &out_path).unwrap();
        }
    }

    // println!("Writing lexicon...");

    // let lexicon = lexicon.into_readonly();

    // if write_txt {}
    // if write_bin {}

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
