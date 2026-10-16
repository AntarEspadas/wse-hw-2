mod doc_score;
mod query;

use std::{cmp::Reverse, collections::BinaryHeap, error::Error};

use common::index::{SCORE_SCALE, decode_score};

use crate::index::{Index, PostingReader};

pub use doc_score::DocScore;
pub use query::Query;

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
        &self,
        query: &Query,
        t: &QueryType,
        top_k: usize,
        score_threshold: f32,
    ) -> Result<Vec<DocScore>, Box<dyn Error>> {
        let terms: Vec<_> = query.terms().iter().cloned().collect();

        let mut heap = match t {
            QueryType::CONJUNCTIVE => self.get_top_docs_conjunctive(&terms, score_threshold)?,
            QueryType::DISJUNCTIVE => self.get_top_docs_disjunctive(&terms, score_threshold)?,
        };

        let result: Vec<_> = (0..top_k).filter_map(|_| heap.pop()).collect();
        Ok(result)
    }

    pub fn normalize(token: &str) -> Option<String> {
        let mut t = token.to_lowercase();

        // Remove punctuation
        t.retain(|c| c.is_alphanumeric());

        if t.is_empty() { None } else { Some(t) }
    }

    fn get_top_docs_conjunctive(
        &self,
        terms: &[String],
        score_threshold: f32,
    ) -> Result<BinaryHeap<DocScore>, Box<dyn Error>> {
        let mut result = BinaryHeap::new();
        let mut current_doc_id = 0u32;

        let mut posting_readers = self.index.get_all_postings(terms)?;

        if terms.is_empty() || posting_readers.len() < terms.len() {
            return Ok(result);
        }

        posting_readers.sort_unstable_by_key(PostingReader::len);

        'outer: loop {
            for posting_reader in posting_readers.iter_mut() {
                let Some(doc_id) = posting_reader.get_next_geq(current_doc_id) else {
                    return Ok(result);
                };

                if doc_id != current_doc_id {
                    current_doc_id = doc_id;
                    continue 'outer;
                }
            }

            let score: i64 = posting_readers
                .iter_mut()
                .map(|reader| decode_score(reader.get_current_score()))
                .sum();

            if score as f32 / SCORE_SCALE >= score_threshold {
                result.push(DocScore {
                    id: current_doc_id,
                    score,
                });
            }

            current_doc_id += 1;
        }
    }

    fn get_top_docs_disjunctive(
        &self,
        terms: &[String],
        score_threshold: f32,
    ) -> Result<BinaryHeap<DocScore>, Box<dyn Error>> {
        let mut result = BinaryHeap::new();
        let mut heap = BinaryHeap::new();
        let mut scores: Vec<DocScore> = Vec::new();

        let mut essential = Vec::new();
        let mut nonessential = Vec::new();

        let mut posting_readers = self.index.get_all_postings(terms)?;

        posting_readers.sort_unstable_by_key(PostingReader::get_max_score);

        let mut score_sum = 0i64;

        for posting_reader in posting_readers {
            // A document can omit this term, so zero is an upper bound when
            // every impact in the list is negative.
            score_sum += decode_score(posting_reader.get_max_score()).max(0);

            if (score_sum as f32 / SCORE_SCALE) < score_threshold {
                nonessential.push(posting_reader);
            } else {
                essential.push(posting_reader);
            }
        }

        for (i, posting_reader) in essential.iter_mut().enumerate() {
            if let Some(doc_id) = posting_reader.get_next_geq(0) {
                heap.push(Reverse((doc_id, i)));
            }
        }

        while let Some(Reverse((current_id, i))) = heap.pop() {
            let current_score = decode_score(essential[i].get_current_score());
            if let Some(doc_score) = scores.last_mut()
                && doc_score.id == current_id
            {
                doc_score.score += current_score;
            } else {
                scores.push(DocScore {
                    id: current_id,
                    score: current_score,
                });
            }
            if let Some(doc_id) = essential[i].get_next_geq(current_id + 1) {
                heap.push(Reverse((doc_id, i)));
            }
        }

        for mut doc_score in scores {
            for posting_reader in nonessential.iter_mut() {
                if let Some(doc_id) = posting_reader.get_next_geq(doc_score.id)
                    && doc_score.id == doc_id
                {
                    doc_score.score += decode_score(posting_reader.get_current_score());
                }
            }

            if doc_score.original_score() >= score_threshold {
                result.push(doc_score);
            }
        }

        Ok(result)
    }
}
