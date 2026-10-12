use std::{error::Error, path::Path};

use query::{
    index::{Index, Lexicon},
    processing::{QueryProcessor, QueryType},
};

fn main() -> Result<(), Box<dyn Error>> {
    println!("New:");
    find_best_new()?;
    Ok(())
}

fn find_best_new() -> Result<(), Box<dyn Error>> {
    let lexicon_path = Path::new("F:\\output\\lexicon.bin");
    let index_path = Path::new("F:\\output\\index.bin");

    println!("Building lexicon...");
    let lexicon = Lexicon::read(lexicon_path)?;
    println!("Done.");

    let index = Index::open(index_path, lexicon)?;

    let mut query_processor = QueryProcessor::new(index);

    let top_results = query_processor.perform_query("the and it", QueryType::CONJUNCTIVE, 10)?;

    println!("Top results: {top_results:?}");

    Ok(())
}
