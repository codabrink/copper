use anyhow::Result;
use serde::Deserialize;
use sqlx::{postgres::PgDatabaseError, query, Database, PgPool};
use std::collections::HashMap;
use tracing::{info, warn};

#[derive(Deserialize)]
pub struct Symbol {
  symbol: String,
  // possible vaules: TRADING, BREAK
  status: String,
  #[serde(rename = "baseAsset")]
  base_asset: String,
  #[serde(rename = "quoteAsset")]
  quote_asset: String,
}

impl Symbol {
  pub async fn populate_all(pool: &PgPool) -> Result<()> {
    let resp = reqwest::get("https://api.binance.com/api/v3/exchangeInfo")
      .await?
      .text()
      .await?;
    let resp: ExchangeInfoResponse = serde_json::from_str(&resp).unwrap();

    for symbol in resp.symbols {
      let result = query!(
        r#"--sql
INSERT INTO symbols
( symbol, status, base_asset, quote_asset )
VALUES ( $1, $2, $3, $4 );
          "#,
        symbol.symbol,
        symbol.status,
        symbol.base_asset,
        symbol.quote_asset
      )
      .execute(pool)
      .await;

      match result {
        Err(err) => {
          if let sqlx::Error::Database(db_err) = &err {
            if let Some(cons) = db_err.constraint() {
              if cons == "symbols_pkey" {
                warn!("{} already exists in database.", symbol.symbol);
                continue;
              }
            }
          }
          Err(err)?;
        }
        _ => {
          info!("Saved symbol {}", symbol.symbol);
        }
      }
    }

    Ok(())
  }
}

#[derive(Deserialize)]
struct ExchangeInfoResponse {
  symbols: Vec<Symbol>,
}
