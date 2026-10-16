use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    api::dto::{QueryType, SearchQuery, SearchResults},
    services::document_service::search_documents,
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

    let result = search_documents(&state, &params.q, &query_type)
        .await
        .unwrap();

    Json(result)
}
