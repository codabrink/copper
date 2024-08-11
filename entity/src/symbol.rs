use anyhow::Result;
use serde::Deserialize;
use sqlx::{postgres::PgDatabaseError, query, query_as, Database, PgPool};
use std::collections::HashMap;
use tracing::{info, warn};

#[derive(Deserialize)]
pub struct Symbol {
  pub symbol: String,
  // possible vaules: TRADING, BREAK
  pub status: String,
  #[serde(rename = "baseAsset")]
  pub base_asset: String,
  #[serde(rename = "quoteAsset")]
  pub quote_asset: String,
}

impl Symbol {
  pub async fn fetch_all(pool: &PgPool) -> Result<Vec<Self>> {
    let symbols = query_as!(
      Self,
      r#"--sql
SELECT * FROM symbols s WHERE s.status = 'TRADING';
      "#
    )
    .fetch_all(pool)
    .await?;

    Ok(symbols)
  }

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
