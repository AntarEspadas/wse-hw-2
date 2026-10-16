use std::{
    error::Error,
    io::{self, BufRead, Write},
    path::Path,
};

use query::{
    index::{Index, Lexicon},
    processing::{DocScore, Query, QueryProcessor, QueryType},
};

fn main() -> Result<(), Box<dyn Error>> {
    let lexicon_path = Path::new("F:\\output\\lexicon.bin");
    let index_path = Path::new("F:\\output\\index.bin");

    println!("Reading lexicon...");
    let lexicon = Lexicon::read(lexicon_path)?;

    let index = Index::new(index_path.to_owned(), lexicon);
    let query_processor = QueryProcessor::new(index);

    let mut query_type = QueryType::DISJUNCTIVE;

    show_prompt(&query_type);

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line == "/exit" || line == "/quit" {
            break;
        } else if line == "/conjunctive" || line == "/con" {
            query_type = QueryType::CONJUNCTIVE
        } else if line == "/disjunctive" || line == "/dis" {
            query_type = QueryType::DISJUNCTIVE
        } else {
            let query = Query::new(&line);
            let results = query_processor.perform_query(&query, &query_type, 10, 5f32)?;
            show_results(&results);
        }

        show_prompt(&query_type);
    }
    Ok(())
}

fn show_prompt(query_type: &QueryType) {
    let query_type = match query_type {
        QueryType::CONJUNCTIVE => "Conjunctive",
        QueryType::DISJUNCTIVE => "Disjunctive",
    };

    print!("{query_type} query: ");
    io::stdout().flush().unwrap();
}

fn show_results(results: &[DocScore]) {
    if results.is_empty() {
        println!("No results found");
        return;
    }

    println!("===== Results =====");
    for result in results {
        println!("Doc id: {}", result.id);
        println!("Score: {}", result.original_score());
        println!("-------------")
    }

    println!()
}
