use std::{error::Error, path::Path};

use sqlx::PgPool;
use tokio::fs::File;

use crate::data::documents::models::Document;

pub async fn get_documents(pool: &PgPool, ids: &[i32]) -> Result<Vec<Document>, Box<dyn Error>> {
    let documents = sqlx::query_as!(
        Document,
        r#"SELECT * FROM documents WHERE id = ANY($1)"#,
        ids
    )
    .fetch_all(pool)
    .await?;

    Ok(documents)
}

pub async fn import_documents(
    pool: &PgPool,
    path: impl AsRef<Path>,
) -> Result<u64, Box<dyn Error>> {
    let file = File::open(path).await?;

    let mut conn = pool.acquire().await?;

    let mut copy = conn
        .copy_in_raw(
            r#"
        COPY documents (id, body) FROM STDIN 
        WITH (FORMAT CSV, DELIMITER E'\t', HEADER FALSE)
        "#,
        )
        .await?;

    copy.read_from(file).await?;

    let rows = copy.finish().await?;

    Ok(rows)
}
