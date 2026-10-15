use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    api::dto::{QueryType, Result, SearchQuery, SearchResults},
    data::documents::queries::get_documents,
    state::AppState,
};

pub async fn get_search(
    Query(params): Query<SearchQuery>,
    State(state): State<AppState>,
) -> Json<SearchResults> {
    let query_type = match params.t {
        QueryType::Con => query::processing::QueryType::CONJUNCTIVE,
        QueryType::Dis => query::processing::QueryType::CONJUNCTIVE,
    };

    let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let results = state
        .query_processor
        .perform_query(&params.q, &query_type, 10, 5.0)
        .unwrap();
    let end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    let elapsed = end - start;

    let ids: Vec<_> = results.iter().map(|x| x.id as i32).collect();

    let documents = get_documents(&state.pool, &ids).await.unwrap();

    let results: Vec<_> = results
        .iter()
        .zip(documents)
        .map(|(x, y)| Result {
            doc_id: x.id,
            score: x.original_score(),
            snippet: y.body,
        })
        .collect();

    Json(SearchResults {
        results,
        time_taken: format!("{elapsed:?}"),
    })
}
