use query::processing::{Query, QueryType};
use std::{
    collections::VecDeque,
    error::Error,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    api::dto::{DocumentResult, SearchResults},
    data::documents::{models::Document, queries},
    state::AppState,
};

pub async fn search_documents(
    state: &AppState,
    query: &str,
    query_type: &QueryType,
) -> Result<SearchResults, Box<dyn Error>> {
    let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let query = Query::new(query);
    let results = state
        .query_processor
        .perform_query(&query, query_type, 10, 5.0)
        .unwrap();
    let end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    let elapsed = end - start;

    let ids: Vec<_> = results.iter().map(|x| x.id as i32).collect();

    let snippets: Vec<_> = queries::get_documents(&state.pool, &ids)
        .await
        .unwrap()
        .into_iter()
        .map(|doc| generate_snippet(&query, doc, 25))
        .collect();

    let results: Vec<_> = results
        .iter()
        .zip(snippets)
        .map(|(doc, snip)| DocumentResult {
            doc_id: doc.id,
            score: doc.original_score(),
            snippet: snip.0,
            term_indices: snip.1,
        })
        .collect();

    Ok(SearchResults {
        results,
        time_taken: format!("{elapsed:?}"),
    })
}

fn generate_snippet(query: &Query, document: Document, len: usize) -> (String, Vec<usize>) {
    let tokens: Vec<_> = document.body.split_whitespace().collect();

    let mut current_term_indices: VecDeque<_> = tokens
        .iter()
        .take(len)
        .enumerate()
        .filter(|(_, x)| query.contains(x))
        .map(|(i, _)| i)
        .collect();

    if tokens.len() <= len {
        let indices: Vec<usize> = current_term_indices.into_iter().collect();
        return (document.body, indices);
    }

    let mut start = 0usize;
    let mut end = len;

    let mut best_window = &tokens[0..len];

    let mut best_indices = current_term_indices.clone();

    let mut window_start = 0usize;

    while end < tokens.len() {
        if let Some(index) = current_term_indices.front().cloned()
            && index == start
        {
            current_term_indices.pop_front();
        }
        if query.contains(tokens[end]) {
            current_term_indices.push_back(end);
        }

        start += 1;
        end += 1;

        if current_term_indices.len() > best_indices.len() {
            best_indices = current_term_indices.clone();
            best_window = &tokens[start..start + len];

            window_start = start;

            // if end >= tokens.len() {
            //     is_window_at_end = true;
            // }
        }
    }

    let result = best_window.join(" ");

    let start_dots = if window_start == 0 { "" } else { "..." };
    let end_dots = if (window_start + len) >= tokens.len() {
        ""
    } else {
        "..."
    };

    let snippet = format!("{start_dots}{result}{end_dots}");
    let indices: Vec<_> = best_indices.into_iter().map(|x| x - window_start).collect();
    (snippet, indices)
}
