CREATE TABLE IF NOT EXISTS candles (
  symbol       TEXT NOT NULL,
  interval     TEXT NOT NULL,
  open_time    INTEGER NOT NULL,
  open         REAL NOT NULL,
  close        REAL NOT NULL,
  high         REAL NOT NULL,
  low          REAL NOT NULL,
  num_trades   INTEGER NOT NULL,
  volume       REAL NOT NULL,
  taker_volume REAL NOT NULL,
  PRIMARY KEY(symbol, interval, open_time)
);



CREATE TABLE IF NOT EXISTS symbols (
  symbol      TEXT PRIMARY KEY,
  status      TEXT NOT NULL,
  base_asset  TEXT NOT NULL,
  quote_asset TEXT NOT NULL
);
