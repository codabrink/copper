use anyhow::Result;
use sqlx::PgPool;
use std::env;

pub async fn pool() -> Result<PgPool> {
  Ok(PgPool::connect(&env::var("DATABASE_URL").unwrap()).await?)
}
