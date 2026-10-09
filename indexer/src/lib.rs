use crossbeam_channel::{Receiver, Sender};
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::{iter, println, vec};

use crate::data::{
    InMessage, Index, IndexMessage, Lexicon, MergeMessage, OutMessage, ReadonlyIndex, TermCounter,
};

pub mod cli;
pub mod data;

pub fn term(token: &str) -> String {
    let mut t = token.to_lowercase();

    // Remove punctuation
    t.retain(|c| c.is_alphanumeric());

    t
}

pub fn write_index_plaintext<I>(sorted_index: I, out_path: &Path) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = (String, (u32, u32))>,
{
     sorted_index.
    println!(
        "Writing index of size {} to {out_path:?}",
        sorted_index.size()
    );

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
            InMessage::Index(data) => {
                println!("[{worker_id}] Start processing data...");
                // data.index.clear();
                let result = process_buffer(&data.buffer, &mut term_counter, &data.index);
                println!("[{worker_id}] Done processing data");
                match result {
                    Ok(()) => tx.send(OutMessage::IndexDone(data))?,
                    Err(_) => tx.send(OutMessage::IndexError(data))?,
                };
            }
            InMessage::Merge(mut data) => {
                // println!("[{worker_id}] Merging indices...");
                // merge_indeces(&mut data.src, &mut data.dest);
                // println!("[{worker_id}] Done merging indices");
                // tx.send(OutMessage::MergeDone(data))?;
            }
            InMessage::Done => return Ok(()),
        }
    }
}

fn process_buffer(
    buffer: &[u8],
    term_counter: &mut TermCounter,
    index: &Index,
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

// fn merge_indeces(src: &mut Index, dest: &mut Index) {
//     for (term, mut postings) in src.drain() {
//         dest.entry(term)
//             .and_modify(|p| p.append(&mut postings))
//             .or_insert_with(|| postings);
//     }
// }

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

    let mut index = Arc::new(Index::new());

    let mut buffers: Vec<_> = iter::repeat_with(|| Vec::with_capacity(buffer_size))
        .take(workers)
        .collect();

    for i in 0.. {
        let mut read = 0;
        for (w, mut buffer) in buffers.drain(..).enumerate() {
            println!("Reading {buffer_size} bytes for worker {w}...");
            buffer.clear();
            read += read_lines_into_buffer(&mut reader, &mut buffer, buffer_size)?;
            println!("Done reading {} bytes for worker {w}", buffer.len());

            tx.send(InMessage::Index(IndexMessage {
                buffer,
                index: Arc::clone(&index),
            }))?;
        }

        if read == 0 {
            break;
        }

        let mut running_jobs = workers;
        let mut last_done_index: Option<Index> = None;

        while running_jobs > 0 {
            let message = rx.recv()?;

            running_jobs -= 1;

            println!("Running jobs: {running_jobs}");
            match message {
                OutMessage::IndexDone(data) => {
                    buffers.push(data.buffer);

                    // if let Some(index) = last_done_index {
                    //     tx.send(InMessage::Merge(MergeMessage {
                    //         src: index,
                    //         dest: data.index,
                    //     }))?;
                    //     last_done_index = None;
                    //     running_jobs += 1;
                    // } else {
                    //     last_done_index = Some(data.index);
                    // }
                }
                OutMessage::MergeDone(data) => {
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
                    buffers.push(data.buffer);
                    println!("ERROR");
                }
            };
        }

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

        let readonly_index = Arc::try_unwrap(index).unwrap().into_read_only();

        add_to_lexion(&readonly_index, lexicon);

        let mut index_vec: Vec<_> = readonly_index.into_inner().into_iter().collect();

        index_vec.sort_unstable_by(|x, y| x.0.cmp(&y.0));

        write_index_plaintext(&readonly_index, &out_path_txt)?;
        write_index_bin(&readonly_index, &out_path)?;

        index.clear();
    }

    for _ in 0..workers {
        tx.send(InMessage::Done)?;
    }

    Ok(())
}

fn add_to_lexion(index: &ReadonlyIndex, lexicon: &Lexicon) {
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
