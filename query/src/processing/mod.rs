mod doc_score;

use std::{collections::BinaryHeap, error::Error};

use crate::{
    index::{Index, PostingReader},
    processing::doc_score::DocScore,
};
fn term(token: &str) -> Option<String> {
    let mut t = token.to_lowercase();

    // Remove punctuation
    t.retain(|c| c.is_alphanumeric());

    if t.is_empty() { None } else { Some(t) }
}
pub enum QueryType {
    CONJUNCTIVE,
    DISJUNCTIVE,
}
pub struct QueryProcessor {
    index: Index,
}

impl QueryProcessor {
    pub fn new(index: Index) -> Self {
        Self { index }
    }

    pub fn perform_query(
        &mut self,
        query: &str,
        t: QueryType,
        top_k: usize,
    ) -> Result<Vec<DocScore>, Box<dyn Error>> {
        let terms: Vec<_> = query.split_whitespace().flat_map(term).collect();

        let mut posting_readers = self.index.get_all_postings(&terms)?;

        let mut heap = match t {
            QueryType::CONJUNCTIVE => self.get_top_docs_conjunctive(&mut posting_readers),
            QueryType::DISJUNCTIVE => panic!("Disjunctive queries not implemented"),
        };

        let result: Vec<_> = (0..top_k).filter_map(|_| heap.pop()).collect();
        Ok(result)
    }

    fn get_top_docs_conjunctive(
        &self,
        posting_readers: &mut [PostingReader],
    ) -> BinaryHeap<DocScore> {
        let mut result = BinaryHeap::new();
        let mut current_doc_id = 0u32;

        posting_readers.sort_unstable_by_key(PostingReader::len);

        'outer: loop {
            for posting_reader in posting_readers.iter_mut() {
                let Some(doc_id) = posting_reader.get_next_geq(current_doc_id) else {
                    return result;
                };

                if doc_id != current_doc_id {
                    current_doc_id = doc_id;
                    continue 'outer;
                }
            }

            let score: f32 = posting_readers
                .iter()
                .map(PostingReader::get_current_score)
                .sum();

            result.push(DocScore {
                id: current_doc_id,
                score,
            });

            current_doc_id += 1;
        }
    }
}
