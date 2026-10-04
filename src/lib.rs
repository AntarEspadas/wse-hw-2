use std::fs::OpenOptions;
use std::io::{BufWriter, Error, Write};
use std::{collections::HashMap, path::Path};

pub mod data;

pub type Index = HashMap<String, Vec<(usize, usize)>>;

pub fn term(token: &str) -> String {
    let mut t = token.to_lowercase();

    // Remove punctuation
    t.retain(|c| c.is_alphanumeric());

    t
}

pub fn write_index_plaintext(
    index: &Index,
    out_folder: &Path,
    index_num: usize,
) -> Result<(), Error> {
    let filename = format!("index-{index_num}.txt");
    let out_path = out_folder.join(filename);
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

    Ok(())
}
