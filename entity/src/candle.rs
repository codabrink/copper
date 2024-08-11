use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Candle {
  symbol: String,
  interval: String,
  open_time: i32,
  open: f32,
  close: f32,
  high: f32,
  low: f32,
  num_trades: i32,
  volume: f32,
  taker_volume: f32,
}

impl Candle {}
