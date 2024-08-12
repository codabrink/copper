use anyhow::{bail, Result};
use csv::StringRecord;
use entity::{Candle, Symbol};
use serde::Deserialize;
use sqlx::PgPool;
use std::{
  io::{BufRead, BufReader},
  ops::{Range, RangeInclusive},
  path::PathBuf,
};
use tokio::{
  fs::{create_dir_all, File},
  io::AsyncWriteExt,
  spawn,
};
use tokio_stream::StreamExt;
use tracing::{error, info};
use zip::ZipArchive;

const INTERVALS: &[&str] = &["15m", "30m", "1h", "2h", "4h", "12h", "1d", "1w", "1mo"];
const BASEURL: &str = "https://data.binance.vision/data/spot/monthly/klines";
static YEARS: RangeInclusive<i32> = 2017..=2024;
static MONTHS: RangeInclusive<i32> = 1..=12;

#[derive(Deserialize)]
struct CSVCandle {
  open_time: i32,
  open: f32,
  close: f32,
  high: f32,
  low: f32,
  num_trades: i32,
  volume: f32,
  taker_volume: f32,
}

pub async fn load_all(pool: &PgPool) -> Result<()> {
  let symbols = Symbol::fetch_all(pool).await?;

  for symbol in symbols {
    info!("Loading {}...", &symbol.symbol);

    let dir = PathBuf::from("history").join(&symbol.symbol);
    for interval in INTERVALS {
      let mut tx = pool.begin().await?;

      let mut candle = Candle {
        symbol: symbol.symbol.clone(),
        interval: interval.to_string(),
        ..Default::default()
      };

      let dir = dir.join(interval);
      for year in YEARS.clone() {
        for month in MONTHS.clone() {
          let zip_path = dir.join(format!("{year}-{month:02}.zip"));
          info!("{zip_path:?}");
          if !zip_path.exists() {
            continue;
          }
          let csv_path = format!("{}-{interval}-{year}-{month:02}.csv", &symbol.symbol);

          let zip_file = std::fs::File::open(zip_path)?;
          let mut archive = ZipArchive::new(zip_file)?;
          let csv = archive.by_name(&csv_path)?;

          let mut reader = BufReader::new(csv);
          let mut buf = Vec::new();

          loop {
            let len = reader.read_until(b'\n', &mut buf)?;
            if len == 0 {
              break;
            }

            let row = String::from_utf8_lossy(&buf[..len]);
            let split: Vec<&str> = row.split(",").collect();

            candle.open_time = split[0].parse()?;
            candle.open = split[1].parse()?;
            candle.high = split[2].parse()?;
            candle.low = split[3].parse()?;
            candle.close = split[4].parse()?;
            candle.volume = split[5].parse()?;
            candle.num_trades = split[8].parse()?;
            candle.taker_volume = split[9].parse()?;

            candle.insert(&mut tx).await?;
          }
        }
      }
    }
  }

  Ok(())
}

pub async fn download_history_all(pool: &PgPool) -> Result<()> {
  let symbols = Symbol::fetch_all(pool).await?;

  for symbol in symbols {
    info!("Downloading {}...", symbol.symbol);
    let mut futures = vec![];
    for interval in INTERVALS {
      for year in YEARS.clone() {
        for month in MONTHS.clone() {
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
