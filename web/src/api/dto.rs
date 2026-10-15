use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryType {
    Con,
    Dis,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub t: QueryType,
}

#[derive(Serialize)]
pub struct SearchResults {
    pub results: Vec<Result>,
    pub time_taken: String,
}

#[derive(Serialize)]
pub struct Result {
    pub doc_id: u32,
    pub score: f32,
}
