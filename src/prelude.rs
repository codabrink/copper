pub use crate::{
  api::{response::*, AppState},
  config::Config,
};
pub use anyhow::Result;
pub use axum::{
  body::Body,
  extract::{Path, State},
  http::StatusCode,
  response::{IntoResponse, Response},
  Extension, Json,
};
pub use entity::*;
pub use serde::{Deserialize, Serialize};
pub use serde_json::json;
pub use serde_with::{serde_as, NoneAsEmptyString};
pub use specta::Type;
pub use std::sync::Arc;
pub use time::Duration;

use color_backtrace::{BacktracePrinter, Verbosity};

pub fn bt_printer() -> BacktracePrinter {
  BacktracePrinter::default()
    .verbosity(Verbosity::Minimal)
    .lib_verbosity(Verbosity::Minimal)
    .strip_function_hash(true)
    .add_frame_filter(Box::new(|frames| {
      frames.retain(|x| {
        let Some(name) = &x.name else {
          return true;
        };
        if name.starts_with("std")
          || name.starts_with("tokio")
          || name.contains("futures_util")
          || name.contains("futures_core")
        {
          return false;
        }
        true
      })
    }))
}

pub fn respond<T, E>(body: T) -> Result<ApiResponse<T>, E>
where
  E: Into<ApiErr>,
{
  Ok(ApiResponse { body })
}
