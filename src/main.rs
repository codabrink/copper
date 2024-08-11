#![feature(try_trait_v2)]

use anyhow::Result;
use clap::Parser;
use entity::Symbol;

mod api;
mod config;
mod db;
mod historical;
mod prelude;

#[tokio::main]
async fn main() -> Result<()> {
  let args = Args::parse();

  tracing_subscriber::fmt()
    .with_file(true)
    .with_line_number(true)
    .with_max_level(tracing::Level::INFO)
    .init();

  let pool = db::pool().await?;

  if args.populate_symbols {
    Symbol::populate_all(&pool).await?;
    return Ok(());
  }

  if args.download_historical {
    historical::download_historical_all(&pool).await?;
  }

  api::serve().await.unwrap();

  Ok(())
}

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
  /// Populate the trading symbols from binance
  #[arg(long)]
  populate_symbols: bool,

  /// Download historical data for all of the symbols in the database
  #[arg(long)]
  download_historical: bool,
}
