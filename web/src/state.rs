use std::sync::Arc;

use query::processing::QueryProcessor;
use sqlx::{Pool, Postgres};

#[derive(Clone)]
pub struct AppState {
    pub query_processor: Arc<QueryProcessor>,
    pub pool: Arc<Pool<Postgres>>,
}
