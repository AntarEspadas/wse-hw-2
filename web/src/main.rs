use std::{error::Error, net::ToSocketAddrs, sync::Arc};

use axum::Router;
use query::{
    index::{Index, Lexicon},
    processing::QueryProcessor,
};
use tower_http::services::ServeDir;
use web::{api::get_api_router, data::connect_db, state::AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let lexicon_path = std::path::Path::new("F:\\output\\lexicon.bin");
    let index_path = std::path::Path::new("F:\\output\\index.bin");

    println!("Reading lexicon...");
    let lexicon = Lexicon::read(lexicon_path)?;

    let index = Index::new(index_path.to_owned(), lexicon);
    let query_processor = QueryProcessor::new(index);

    let pool = connect_db().await?;

    let state = AppState {
        query_processor: Arc::new(query_processor),
        pool: Arc::new(pool),
    };

    let api = get_api_router();

    let app = Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new(".output/public"))
        .with_state(state);

    let address = "127.0.0.1:8080".to_socket_addrs()?.next().unwrap();

    println!("Starting server on address http://{address:?}");

    let listener = tokio::net::TcpListener::bind(address).await?;

    axum::serve(listener, app).await?;

    Ok(())
}
