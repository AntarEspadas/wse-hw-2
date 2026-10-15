use std::path::Path;

use web::data::{connect_db, documents::queries::import_documents};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let pool = connect_db().await?;

    let path = Path::new("../data/collection.tsv");

    let imported = import_documents(&pool, path).await?;

    println!("Imported {imported} documents");

    pool.close().await;

    Ok(())
}
