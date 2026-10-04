use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;

use wse_hw_2::Index;
use wse_hw_2::data;
use wse_hw_2::term;
use wse_hw_2::write_index_plaintext;

const OUT_FOLDER: &str = "output";
const IN_PATH: &str = "./data/collection.tsv";
const MAX_LINES: usize = 100_000;

#[derive(Debug, serde::Deserialize)]
struct Document {
    id: usize,
    content: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut index: Index = Index::new();

    let out_folder = PathBuf::from_str(OUT_FOLDER)?;

    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_path(IN_PATH)?;
    let mut index_count = 0;

    let mut term_counter = data::TermCounter::new();
    for (i, result) in csv_reader.deserialize().enumerate() {
        let document: Document = result?;

        for token in document.content.split_ascii_whitespace() {
            let t = term(token);
            term_counter.count(t);
        }

        for (term, count) in term_counter.iter() {
            if let Some(postings) = index.get_mut(term) {
                postings.push((document.id, *count));
            } else {
                index.insert((*term).to_owned(), vec![(document.id, *count)]);
            }
        }

        term_counter.clear();

        if i != 0 && i % MAX_LINES == 0 {
            write_index_plaintext(&index, &out_folder, index_count)?;
            index.clear();
            index_count += 1;
        }
    }
    if !index.is_empty() {
        write_index_plaintext(&index, &out_folder, index_count)?;
    }
    Ok(())
}
