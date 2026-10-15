use std::sync::Arc;

use query::processing::QueryProcessor;

#[derive(Clone)]
pub struct AppState {
    pub query_processor: Arc<QueryProcessor>,
}
