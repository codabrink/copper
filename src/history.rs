use anyhow::{bail, Result};
use async_zip::tokio::read::seek::ZipFileReader;
use entity::{Candle, Symbol};
use futures::StreamExt;
use sqlx::PgPool;
use std::{ops::RangeInclusive, path::PathBuf};
use tokio::{
  fs::{create_dir_all, File},
  io::{AsyncWriteExt, BufReader},
  spawn, stream,
};
use tracing::{error, info};

const INTERVALS: &[&str] = &["15m", "30m", "1h", "2h", "4h", "12h", "1d", "1w", "1mo"];
const BASEURL: &str = "https://data.binance.vision/data/spot/monthly/klines";
static YEARS: RangeInclusive<i32> = 2017..=2024;
static MONTHS: RangeInclusive<i32> = 1..=12;

pub async fn load_all(pool: &PgPool) -> Result<()> {
  let symbols = Symbol::fetch_all(pool).await?;
  let mut futures = vec![];

  // We know it'll last long enough.
  let pool = unsafe { std::mem::transmute::<&'_ PgPool, &'static PgPool>(pool) };

  for symbol in symbols {
    info!("Loading {}...", &symbol.symbol);

    let dir = PathBuf::from("history").join(&symbol.symbol);
    for interval in INTERVALS {
      let dir = dir.join(interval);
      for year in YEARS.clone() {
        futures.push(load_year(
          pool,
          dir.clone(),
          symbol.symbol.clone(),
          interval,
          year,
        ));
      }
    }
  }

  let mut stream_of_futures = futures::stream::iter(futures).buffer_unordered(30);
  while let Some(_) = stream_of_futures.next().await {}

  Ok(())
}

async fn load_year<'a>(
  pool: &'a PgPool,
  dir: PathBuf,
  symbol: String,
  interval: &str,
  year: i32,
) -> Result<()> {
  let mut tx = pool.begin().await?;
  for month in MONTHS.clone() {
    let zip_path = dir.join(format!("{year}-{month:02}.zip"));
    info!("{zip_path:?}");
    if !zip_path.exists() {
      continue;
    }
    // let csv_path = format!("{}-{interval}-{year}-{month:02}.csv", &symbol);

    let mut zip_file = BufReader::new(File::open(zip_path).await?);
    let mut zip = ZipFileReader::with_tokio(&mut zip_file).await?;

    let mut csv_reader = zip.reader_with_entry(0).await?;
    let mut csv = String::new();
    csv_reader.read_to_string_checked(&mut csv).await?;

    let lines: Vec<&str> = csv.split("\n").collect();
    for line in lines {
      if line.len() == 0 {
        continue;
      }

      let split: Vec<&str> = line.split(",").collect();

      let candle = Candle {
        symbol: symbol.to_string(),
        interval: interval.to_string(),
        open_time: split[0].parse()?,
        open: split[1].parse()?,
        high: split[2].parse()?,
        low: split[3].parse()?,
        close: split[4].parse()?,
        volume: split[5].parse()?,
        num_trades: split[8].parse()?,
        taker_volume: split[9].parse()?,
      };

      candle.insert(&mut tx).await?;
    }
  }
  tx.commit().await?;
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
