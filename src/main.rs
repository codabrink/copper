#![feature(try_trait_v2)]

use anyhow::Result;
use clap::Parser;
use entity::Symbol;

mod api;
mod config;
mod db;
mod history;
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
    let mut tx = pool.begin().await?;
    Symbol::populate_all(&mut tx).await?;
    tx.commit().await?;
    return Ok(());
  }

  if args.download_history {
    history::download_history_all(&pool).await?;
    return Ok(());
  }

  if args.load_history {
    history::load_btc_usdt(&pool).await?;
    return Ok(());
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
  download_history: bool,

  /// Load the downloaded candle data into the database
  #[arg(long)]
  load_history: bool,
}
