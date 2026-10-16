use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    api::dto::{SearchQuery, SearchResults},
    services::document_service::search_documents,
    state::AppState,
};

pub async fn get_search(
    Query(params): Query<SearchQuery>,
    State(state): State<AppState>,
) -> Json<SearchResults> {
    let query_type = params.t.into();

    let result = search_documents(&state, &params.q, &query_type)
        .await
        .unwrap();

    Json(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_modes_select_the_corresponding_query_algorithm() {
        let conjunctive = Query::<SearchQuery>::try_from_uri(
            &"/api/search?q=alpha%20beta&t=con".parse().unwrap(),
        )
        .unwrap();
        assert_eq!(conjunctive.q, "alpha beta");
        assert!(matches!(
            conjunctive.0.t.into(),
            query::processing::QueryType::CONJUNCTIVE
        ));

        let disjunctive = Query::<SearchQuery>::try_from_uri(
            &"/api/search?q=alpha%20beta&t=dis".parse().unwrap(),
        )
        .unwrap();
        assert!(matches!(
            disjunctive.0.t.into(),
            query::processing::QueryType::DISJUNCTIVE
        ));
    }
}
