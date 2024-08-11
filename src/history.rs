use anyhow::{bail, Result};
use entity::Symbol;
use sqlx::PgPool;
use std::path::PathBuf;
use tokio::{
  fs::{create_dir_all, File},
  io::AsyncWriteExt,
  spawn,
};
use tokio_stream::StreamExt;
use tracing::{error, info};

const INTERVALS: &[&str] = &["15m", "30m", "1h", "2h", "4h", "12h", "1d", "1w", "1mo"];
const BASEURL: &str = "https://data.binance.vision/data/spot/monthly/klines";

pub async fn download_historical_all(pool: &PgPool) -> Result<()> {
  let symbols = Symbol::fetch_all(pool).await?;

  for symbol in symbols {
    let mut futures = vec![];
    for interval in INTERVALS {
      for year in 2017..=2024 {
        for month in 1..=12 {
          futures.push(spawn(download_month(
            symbol.symbol.clone(),
            *interval,
            year,
            month,
          )));
        }
      }
    }
    for future in futures {
      if let Err(err) = future.await? {
        error!("{err:?}");
      };
    }
  }

  Ok(())
}

pub async fn download_month(symbol: String, interval: &str, year: i32, month: i32) -> Result<()> {
  let dl_dir = PathBuf::from("history").join(&symbol);
  let year = format!("{year}");

  let month = format!("{month:02}");
  let dl_dir = dl_dir.join(interval);

  let _ = create_dir_all(&dl_dir).await;

  let zip = format!("{year}-{month}.zip");
  let url = format!("{BASEURL}/{symbol}/{interval}/{symbol}-{interval}-{year}-{month}.zip");

  let file_path = dl_dir.join(&zip);
  if file_path.exists() {
    return Ok(());
  }

  let resp = reqwest::get(url).await?;
  let status = resp.status();
  if !status.is_success() {
    if status.as_u16() == 404 {
      return Ok(());
    }
    bail!("Server responded {:?} for {zip}", resp.status());
  }

  info!("Downloading {symbol}-{interval}-{zip}...");

  let mut stream = resp.bytes_stream();
  let mut file = File::create(&file_path).await?;

  while let Some(bytes) = stream.next().await {
    file.write(&bytes?).await?;
  }

  Ok(())
}
