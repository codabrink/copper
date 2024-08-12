use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;

pub async fn pool() -> Result<PgPool> {
  Ok(
    PgPoolOptions::new()
      .max_connections(40)
      .connect(&env::var("DATABASE_URL").unwrap())
      .await?,
  )
}
