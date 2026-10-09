use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::{iter, println, thread, vec};

use dashmap::ReadOnlyView;
use parking_lot::Mutex;

use crate::data::{Index, Lexicon, TermCounter};

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
    I: Iterator<Item = &'a mut Vec<(u32, u32)>>,
{
    println!(
        "Writing index of size {} to {out_path:?}",
        index.size_hint().0
    );

    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;
    let mut writer = BufWriter::new(file);

    for (term_id, postings) in index.enumerate() {
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
    I: Iterator<Item = &'a mut Vec<(u32, u32)>>,
{
    println!(
        "Writing index of size {} to {out_path:?}",
        index.size_hint().0
    );

    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)?;
    let mut writer = BufWriter::new(file);

    for (term_id, postings) in index.enumerate() {
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
            let term_id = *lexicon.get(&term).unwrap();

            let mut postings = index[term_id].lock();

            postings.push((doc_id, count));
        }
    }
    println!("[{worker_id}] Done processing data");

    Ok(())
}

fn populate_lexicon(
    buffer: &[u8],
    lexicon: &Lexicon,
    worker_id: usize,
) -> Result<(), Box<dyn Error>> {
    println!("[{worker_id}] Populating lexicon...");

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

        thread::scope(|scope| {
            for (worker_id, buffer) in buffers.iter_mut().enumerate() {
                println!("Reading {buffer_size} bytes for worker {worker_id}...");
                buffer.clear();
                read += read_lines_into_buffer(&mut reader, buffer, buffer_size).unwrap();
                println!("Done reading {} bytes for worker {worker_id}", buffer.len());

                let lexicon = &lexicon;
                scope.spawn(move || {
                    populate_lexicon(buffer, lexicon, worker_id).unwrap();
                });
            }
        });

        let readonly_lexicon = lexicon.into_readonly();
        // let index: Index = iter::repeat_with(|| Mutex::new(Vec::new()))
        //     .take(readonly_lexicon.len())
        //     .collect();

        // thread::scope(|scope| {
        //     for (worker_id, buffer) in buffers.iter_mut().enumerate() {
        //         let index = &index;
        //         let readonly_lexicon = &readonly_lexicon;
        //         scope.spawn(move || {
        //             process_buffer(buffer, index, readonly_lexicon, worker_id).unwrap()
        //         });
        //     }
        // });

        lexicon = Lexicon::from_readonly(readonly_lexicon);

        if read == 0 {
            break;
        }

        // println!(
        //     "Iteration {i}: indexed {} terms (index capacity: {})",
        //     index.len(),
        //     index.capacity()
        // );
        let mut filename = format!("index-{i}");
        let out_path = out_folder.join(&filename);
        filename.push_str(".txt");
        let out_path_txt = out_folder.join(&filename);

        // let mut index = index;

        // for postings in index.iter_mut() {
        //     let postings = postings.get_mut();
        //     postings.sort_unstable_by_key(|x| x.0);
        // }

        // let iterator = index
        //     .iter_mut()
        //     .map(|x| x.get_mut())
        //     .filter(|x| !x.is_empty());

        // write_index_plaintext(iterator, &out_path_txt)?;

        // let iterator = index
        //     .iter_mut()
        //     .map(|x| x.get_mut())
        //     .filter(|x| !x.is_empty());

        // write_index_bin(iterator, &out_path)?;

        println!("Updating lexicon...");

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    println!("Writing lexicon...");

    let lexicon = lexicon.into_readonly();

    let out_path = out_folder.join("lexicon.bin");
    write_lexicon_bin(lexicon.iter(), &out_path).unwrap();
    let out_path = out_folder.join("lexicon.txt");
    write_lexicon_plaintext(lexicon.iter(), &out_path).unwrap();

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
