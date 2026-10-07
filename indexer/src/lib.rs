use crossbeam_channel::{Receiver, Sender};
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::{iter, println, vec};

use crate::data::{
    InMessage, IndexMessage, Lexicon, MergeMessage, OutMessage, OwnedIndex, TermCounter,
};

pub mod cli;
pub mod data;

pub fn term(token: &str) -> String {
    let mut t = token.to_lowercase();

    // Remove punctuation
    t.retain(|c| c.is_alphanumeric());

    t
}

pub fn write_index_plaintext(index: &OwnedIndex, out_path: &Path) -> Result<(), Box<dyn Error>> {
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

pub fn write_index_bin(index: &OwnedIndex, out_path: &Path) -> Result<(), Box<dyn Error>> {
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

pub fn worker(
    rx: &Receiver<InMessage>,
    tx: &Sender<OutMessage>,
    worker_id: usize,
) -> Result<(), Box<dyn Error>> {
    println!("[{worker_id}] Starting worker...");
    let mut term_counter = TermCounter::new();
    loop {
        let message = rx.recv()?;
        match message {
            InMessage::Index(mut data) => {
                println!("[{worker_id}] Start processing data...");
                data.index.clear();
                let result = process_buffer(&data.buffer, &mut term_counter, &mut data.index);
                println!("[{worker_id}] Done processing data");
                match result {
                    Ok(()) => tx.send(OutMessage::IndexDone(data))?,
                    Err(_) => tx.send(OutMessage::IndexError(data))?,
                };
            }
            InMessage::Merge(mut data) => {
                println!("[{worker_id}] Merging indices...");
                merge_indeces(&mut data.src, &mut data.dest);
                println!("[{worker_id}] Done merging indices");
                tx.send(OutMessage::MergeDone(data))?;
            }
            InMessage::Done => return Ok(()),
        }
    }
}

fn process_buffer(
    buffer: &[u8],
    term_counter: &mut TermCounter,
    index: &mut OwnedIndex,
) -> Result<(), Box<dyn Error>> {
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

    Ok(())
}

fn merge_indeces(src: &mut OwnedIndex, dest: &mut OwnedIndex) {
    for (term, mut postings) in src.drain() {
        dest.entry(term)
            .and_modify(|p| p.append(&mut postings))
            .or_insert_with(|| postings);
    }
}

pub fn produce_from_csv(
    tx: &Sender<InMessage>,
    rx: &Receiver<OutMessage>,
    csv_path: &Path,
    chunk_size: usize,
    workers: usize,
    out_folder: &Path,
    lexicon: &Lexicon,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(csv_path)?;

    println!("chunk_size: {chunk_size}, workers: {workers}");
    let buffer_size = chunk_size / workers;

    let mut reader = BufReader::new(file);

    let mut buffers: Vec<_> = iter::repeat_with(|| Vec::with_capacity(buffer_size))
        .take(workers)
        .collect();
    let mut indices = vec![OwnedIndex::new(); workers];

    for i in 0.. {
        let mut read = 0;
        for (w, (mut buffer, index)) in buffers.drain(..).zip(indices.drain(..)).enumerate() {
            println!("Reading {buffer_size} bytes for worker {w}...");
            buffer.clear();
            read += read_lines_into_buffer(&mut reader, &mut buffer, buffer_size)?;
            println!("Done reading {} bytes for worker {w}", buffer.len());

            tx.send(InMessage::Index(IndexMessage { buffer, index }))?;
        }

        if read == 0 {
            break;
        }

        let mut running_jobs = workers;
        let mut last_done_index: Option<OwnedIndex> = None;

        while running_jobs > 0 {
            let message = rx.recv()?;

            running_jobs -= 1;

            println!("Running jobs: {running_jobs}");
            match message {
                OutMessage::IndexDone(data) => {
                    buffers.push(data.buffer);

                    if let Some(index) = last_done_index {
                        tx.send(InMessage::Merge(MergeMessage {
                            src: index,
                            dest: data.index,
                        }))?;
                        last_done_index = None;
                        running_jobs += 1;
                    } else {
                        last_done_index = Some(data.index);
                    }
                }
                OutMessage::MergeDone(data) => {
                    indices.push(data.src);

                    if let Some(index) = last_done_index {
                        tx.send(InMessage::Merge(MergeMessage {
                            src: index,
                            dest: data.dest,
                        }))?;
                        last_done_index = None;
                        running_jobs += 1;
                    } else {
                        last_done_index = Some(data.dest);
                    }
                }
                OutMessage::IndexError(data) => {
                    indices.push(data.index);
                    buffers.push(data.buffer);
                    println!("ERROR");
                }
            };
        }

        let mut index = last_done_index.unwrap();
        println!("Done merging indices");
        println!(
            "Iteration {i}: indexed {} terms (index capacity: {})",
            index.len(),
            index.capacity()
        );
        let mut filename = format!("index-{i}");
        let out_path = out_folder.join(&filename);
        filename.push_str(".txt");
        let out_path_txt = out_folder.join(&filename);
        write_index_plaintext(&index, &out_path_txt)?;
        write_index_bin(&index, &out_path)?;

        add_to_lexion(&mut index, lexicon);

        indices.push(index);
    }

    for _ in 0..workers {
        tx.send(InMessage::Done)?;
    }

    Ok(())
}

fn add_to_lexion(index: &mut OwnedIndex, lexicon: &Lexicon) {
    for (term, _) in index.drain() {
        lexicon.add(term);
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
