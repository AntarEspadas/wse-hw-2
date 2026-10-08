use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::{iter, println, thread, vec};

use crate::data::{Index, Lexicon, ReadonlyIndex, TermCounter};

pub mod cli;
pub mod data;

pub fn term(token: &str) -> String {
    let mut t = token.to_lowercase();

    // Remove punctuation
    t.retain(|c| c.is_alphanumeric());

    t
}

pub fn write_index_plaintext(index: &ReadonlyIndex, out_path: &Path) -> Result<(), Box<dyn Error>> {
    println!("Writing index of size {} to {out_path:?}", index.len());

    let mut index_vec: Vec<_> = index.iter().collect();

    index_vec.sort_unstable_by_key(|x| x.0);

    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;
    let mut writer = BufWriter::new(file);

    for (term, postings) in index_vec.into_iter() {
        write!(writer, "{term};{};", postings.len())?;
        for (doc_id, count) in postings {
            write!(writer, " {} {}", doc_id, count)?;
        }
        writeln!(writer)?;
    }

    writer.flush()?;
    Ok(())
}

pub fn write_index_bin(index: &ReadonlyIndex, out_path: &Path) -> Result<(), Box<dyn Error>> {
    println!("Writing index of size {} to {out_path:?}", index.len());

    let mut index_vec: Vec<_> = index.iter().collect();

    index_vec.sort_unstable_by_key(|x| x.0);

    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;
    let mut writer = BufWriter::new(file);

    for (term, postings) in index_vec.into_iter() {
        write!(writer, "{term}\0")?;
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

pub fn write_lexicon_plaintext(lexicon: &Lexicon, out_path: &Path) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;

    let mut writer = BufWriter::new(file);

    for (term, term_id) in lexicon.iter() {
        writeln!(writer, "{term};{term_id}")?;
    }

    writer.flush()?;
    Ok(())
}

pub fn write_lexicon_bin(lexicon: &Lexicon, out_path: &Path) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;

    let mut writer = BufWriter::new(file);

    for (term, term_id) in lexicon.iter() {
        write!(writer, "{term}\0")?;
        writer.write_all(&term_id.to_be_bytes())?;
    }

    writer.flush()?;
    Ok(())
}

pub fn process_buffer(
    buffer: &[u8],
    index: &Index,
    worker_id: usize,
) -> Result<(), Box<dyn Error>> {
    println!("[{worker_id}] Start processing data...");
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
            let entry = index.entry(term);

            entry
                .and_modify(|postings| postings.push((doc_id, count)))
                .or_insert_with(|| vec![(doc_id, count)]);
        }
    }
    println!("[{worker_id}] Done processing data");

    Ok(())
}

pub fn generate_index(
    csv_path: &Path,
    chunk_size: usize,
    workers: usize,
    out_folder: &Path,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(csv_path)?;
    let lexicon = Lexicon::new();

    println!("chunk_size: {chunk_size}, workers: {workers}");
    let buffer_size = chunk_size / workers;

    let mut reader = BufReader::new(file);

    let mut index = Index::new();

    let mut buffers: Vec<_> = iter::repeat_with(|| Vec::with_capacity(buffer_size))
        .take(workers)
        .collect();

    for i in 0.. {
        let mut read = 0;

        thread::scope(|scope| {
            for (worker_id, buffer) in buffers.iter_mut().enumerate() {
                println!("Reading {buffer_size} bytes for worker {worker_id}...");
                buffer.clear();
                read += read_lines_into_buffer(&mut reader, buffer, buffer_size).unwrap();
                println!("Done reading {} bytes for worker {worker_id}", buffer.len());

                let index = &index;
                scope.spawn(move || process_buffer(buffer, index, worker_id).unwrap());
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
        let mut filename = format!("index-{i}");
        let out_path = out_folder.join(&filename);
        filename.push_str(".txt");
        let out_path_txt = out_folder.join(&filename);

        let readonly_index = index.into_read_only();

        write_index_plaintext(&readonly_index, &out_path_txt)?;
        write_index_bin(&readonly_index, &out_path)?;

        add_to_lexicon(&readonly_index, &lexicon);

        index = readonly_index.into_inner();
        index.clear();
    }

    let out_path = out_folder.join("lexicon.bin");
    write_lexicon_bin(&lexicon, &out_path).unwrap();
    let out_path = out_folder.join("lexicon.txt");
    write_lexicon_plaintext(&lexicon, &out_path).unwrap();

    Ok(())
}

fn add_to_lexicon(index: &ReadonlyIndex, lexicon: &Lexicon) {
    for (term, _) in index.iter() {
        lexicon.add(term.to_owned());
    }
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
