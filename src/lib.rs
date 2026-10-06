use crossbeam_channel::{Receiver, Sender};
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::iter;
use std::path::PathBuf;

use crate::data::{InMessage, Index, MessageData, OutMessage, TermCounter};

pub mod data;

pub fn term(token: &str) -> String {
    let mut t = token.to_lowercase();

    // Remove punctuation
    t.retain(|c| c.is_alphanumeric());

    t
}

pub fn write_index_plaintext(
    index: &Index,
    out_folder: &str,
    index_num: usize,
) -> Result<(), Box<dyn Error>> {
    let filename = format!("index-{index_num}.txt");
    let out_path: PathBuf = [out_folder, &filename].iter().collect();
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
        write!(writer, "{term}")?;
        for (doc_id, count) in postings {
            write!(writer, " {} {}", doc_id, count)?;
        }
        writeln!(writer)?;
    }

    writer.flush()?;
    Ok(())
}

pub fn worker(rx: &Receiver<InMessage>, tx: &Sender<OutMessage>) -> Result<(), Box<dyn Error>> {
    let mut term_counter = TermCounter::new();
    loop {
        let message = rx.recv()?;
        match message {
            InMessage::Data(mut data) => {
                data.index.clear();
                let result = process_buffer(&data.buffer, &mut term_counter, &mut data.index);
                match result {
                    Ok(()) => tx.send(OutMessage::Data(data))?,
                    Err(_) => tx.send(OutMessage::Error(data))?,
                };
            }
            InMessage::Done => return Ok(()),
        }
    }
}

fn process_buffer(
    buffer: &[u8],
    term_counter: &mut TermCounter,
    index: &mut Index,
) -> Result<(), Box<dyn Error>> {
    for line in str::from_utf8(buffer)?.lines() {
        let mut iterator = line.split('\t');
        let doc_id = iterator.next().unwrap();
        let doc_id: usize = doc_id.parse()?;

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

pub fn produce_from_csv(
    tx: &Sender<InMessage>,
    rx: &Receiver<OutMessage>,
    csv_path: &str,
    chunk_size: usize,
    workers: usize,
    out_folder: &str,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(csv_path)?;

    let buffer_size = chunk_size / workers;

    let mut reader = BufReader::new(file);

    let mut messages: Vec<_> = iter::repeat_with(|| MessageData {
        buffer: Vec::with_capacity(buffer_size),
        index: Index::new(),
    })
    .take(workers)
    .collect();

    let mut index = Index::new();

    for i in 0.. {
        let mut read = 0;
        for (w, mut message) in messages.drain(..).enumerate() {
            println!("Reading {buffer_size} bytes for worker {w}...");
            message.buffer.clear();
            read += read_lines_into_buffer(&mut reader, &mut message.buffer, buffer_size)?;
            println!("Done reading {buffer_size} bytes for worker {w}");

            tx.send(InMessage::Data(message))?;
        }

        if read == 0 {
            break;
        }

        index.clear();

        for w in 0..workers {
            let message = rx.recv()?;

            let data = match message {
                OutMessage::Data(mut data) => {
                    merge_index_into(&mut data.index, &mut index);
                    data
                }
                OutMessage::Error(data) => {
                    println!("ERROR in worker {w}");
                    data
                }
            };

            messages.push(data);
        }

        println!("Iteration {i}: indexed {} terms", index.len());
        write_index_plaintext(&index, out_folder, i)?;
    }

    for _ in 0..workers {
        tx.send(InMessage::Done)?;
    }

    Ok(())
}

fn merge_index_into(source: &mut Index, dest: &mut Index) {
    for (term, source_postings) in source.drain() {
        dest.entry(term)
            .and_modify(|dest_postings| dest_postings.extend(source_postings.iter()))
            .or_insert(source_postings);
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
