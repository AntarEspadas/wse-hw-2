use std::{env, error::Error};

use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

pub mod documents;

pub async fn connect_db() -> Result<Pool<Postgres>, Box<dyn Error>> {
    let database_url = env::var("DATABASE_URL")?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .connect(&database_url)
        .await?;

    Ok(pool)
}
