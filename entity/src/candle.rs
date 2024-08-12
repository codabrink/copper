use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgConnection};

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Candle {
  pub symbol: String,
  pub interval: String,
  pub open_time: i64,
  pub open: f32,
  pub close: f32,
  pub high: f32,
  pub low: f32,
  pub num_trades: i32,
  pub volume: f32,
  pub taker_volume: f32,
}

impl Candle {
  pub async fn insert(&self, pool: &mut PgConnection) -> Result<()> {
    query!(
      r#"--sql
INSERT INTO candles
( symbol, interval, open_time, open, close, high, low, num_trades, volume, taker_volume )
VALUES ( $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 )
ON CONFLICT ( symbol, "interval", open_time ) DO NOTHING;
      "#,
      self.symbol,
      self.interval,
      self.open_time,
      self.open,
      self.close,
      self.high,
      self.low,
      self.num_trades,
      self.volume,
      self.taker_volume
    )
    .execute(pool)
    .await?;

    Ok(())
  }
}
