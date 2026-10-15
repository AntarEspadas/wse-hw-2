use std::{net::ToSocketAddrs, sync::Arc};

use axum::Router;
use query::{
    index::{Index, Lexicon},
    processing::QueryProcessor,
};
use tower_http::services::ServeDir;

use crate::{api::get_api_router, state::AppState};

mod api;
mod state;

#[tokio::main]
async fn main() {
    let lexicon_path = std::path::Path::new("F:\\output\\lexicon.bin");
    let index_path = std::path::Path::new("F:\\output\\index.bin");

    println!("Reading lexicon...");
    let lexicon = Lexicon::read(lexicon_path).unwrap();

    let index = Index::new(index_path.to_owned(), lexicon);
    let query_processor = QueryProcessor::new(index);

    let state = AppState {
        query_processor: Arc::new(query_processor),
    };

    let api = get_api_router();

    let app = Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new(".output/public"))
        .with_state(state);

    let address = "127.0.0.1:8080".to_socket_addrs().unwrap().next().unwrap();

    println!("Starting server on address http://{address:?}");

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
