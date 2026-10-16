use std::env;

use web::data::{connect_db, documents::queries::import_documents};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let path = env::var("COLLECTION_PATH")?;

    let pool = connect_db().await?;

    let imported = import_documents(&pool, path).await?;

    println!("Imported {imported} documents");

    pool.close().await;

    Ok(())
}
