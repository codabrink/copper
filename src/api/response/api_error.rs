use anyhow::Result;
use axum::{
  http::{header, StatusCode},
  response::IntoResponse,
  Json,
};
use backtrace::Backtrace;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::{
  collections::HashMap,
  ops::Try,
  ops::{ControlFlow, FromResidual},
};
use tokio::task::JoinError;
use tracing::{event, Level};

use crate::api::bt_printer;

use super::field_errors::FieldErrors;

static UNIQUE_CONSTRAINT_REGEX: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"Key\s\((?<names>.+?)\)").unwrap());

/// This trait makes errors "api friendly"
/// Converts errors into something the client can consume
/// And logs the error (with potential backtrace) for debugging
pub trait ToApiErr<T> {
  fn api(self) -> PossibleApiErr<T>;
}

impl<T> ToApiErr<T> for Result<T, sqlx::Error> {
  fn api(self) -> PossibleApiErr<T> {
    match self {
      Ok(t) => PossibleApiErr::Ok(t),
      Err(err) => {
        let err = ErrResidual::new(err.to_string());
        // TODO: Attempt to extract field errors
        PossibleApiErr::Err(err).pub_msg("Unable to save record")
      }
    }
  }
}

// handle implementations for different error results here
// these trait implementations describe how we want the api to respond
// nicely to different errors or problems
//impl<T> ToApiErr<T> for Result<T, DbErr> {
//  fn api(self) -> PossibleApiErr<T> {
//    // attempt_db_constraint_formatting(self)
//    match self {
//      Ok(t) => PossibleApiErr::Ok(t),
//      Err(err) => {
//        let (err, field_errors) = extract_fielderrors(err);
//        let mut err = ErrResidual::new(err.to_string());
//        err.field_errors = Some(field_errors);
//        PossibleApiErr::Err(err).pub_msg("Unable to save record")
//      }
//    }
//  }
//}
impl<T> ToApiErr<T> for Result<T, JoinError> {
  fn api(self) -> PossibleApiErr<T> {
    match self {
      Ok(t) => PossibleApiErr::Ok(t),
      Err(err) => PossibleApiErr::new_err(err.to_string()),
    }
  }
}
impl<T> ToApiErr<T> for Result<T, anyhow::Error> {
  fn api(self) -> PossibleApiErr<T> {
    match self {
      Ok(t) => PossibleApiErr::Ok(t),
      Err(err) => PossibleApiErr::new_err(err.to_string()),
    }
  }
}

impl<T> ToApiErr<T> for Result<T, reqwest::Error> {
  fn api(self) -> PossibleApiErr<T> {
    match self {
      Ok(t) => PossibleApiErr::Ok(t),
      Err(err) => PossibleApiErr::new_err(err.to_string()),
    }
  }
}
impl<T> ToApiErr<T> for Result<T, serde_json::Error> {
  fn api(self) -> PossibleApiErr<T> {
    match self {
      Ok(t) => PossibleApiErr::Ok(t),
      Err(err) => PossibleApiErr::new_err(err.to_string()),
    }
  }
}

impl<T> ToApiErr<T> for Result<T, base64::DecodeError> {
  fn api(self) -> PossibleApiErr<T> {
    match self {
      Ok(t) => PossibleApiErr::Ok(t),
      Err(err) => PossibleApiErr::new_err(err.to_string())
        .pub_msg("There was a problem parsing the attached file"),
    }
  }
}
impl<T> ToApiErr<T> for Option<T> {
  fn api(self) -> PossibleApiErr<T> {
    match self {
      Some(v) => PossibleApiErr::Ok(v),
      None => PossibleApiErr::new_err("Record not found")
        .without_backtrace()
        .info(),
    }
  }
}

#[derive(Debug, Serialize, specta::Type)]
pub struct ApiErr {
  pub message: String,
  #[serde(skip_serializing)]
  pub status_code: StatusCode,
  pub field_errors: Option<HashMap<String, Vec<String>>>,
}
impl IntoResponse for ApiErr {
  fn into_response(self) -> axum::response::Response {
    (
      self.status_code,
      [(header::CONTENT_TYPE, "application/json")],
      Json(self),
    )
      .into_response()
  }
}

pub enum PossibleApiErr<T> {
  Ok(T),
  Err(ErrResidual),
}

impl<T> PossibleApiErr<T> {
  fn new_err(msg: impl ToString) -> Self {
    PossibleApiErr::Err(ErrResidual::new(msg))
  }
  pub fn status_code(mut self, code: StatusCode) -> Self {
    if let Self::Err(err) = &mut self {
      err.status_code = code
    }
    self
  }
  pub fn pub_msg(mut self, msg: impl ToString) -> Self {
    if let Self::Err(err) = &mut self {
      err.pub_msg = msg.to_string();
    }
    self
  }
  pub fn context(mut self, msg: impl ToString) -> Self {
    if let Self::Err(err) = &mut self {
      err.context = Some(msg.to_string());
    }
    self
  }
  pub fn without_backtrace(mut self) -> Self {
    if let Self::Err(err) = &mut self {
      err.with_bt = false;
    }
    self
  }
  pub fn warn(mut self) -> Self {
    if let Self::Err(err) = &mut self {
      err.lvl = Lvl::WARN;
    }
    self
  }
  pub fn info(mut self) -> Self {
    if let Self::Err(err) = &mut self {
      err.lvl = Lvl::INFO;
    }
    self
  }
}
pub struct ErrResidual {
  err_msg: String,
  pub_msg: String,
  field_errors: Option<FieldErrors>,
  with_bt: bool,
  context: Option<String>,
  status_code: StatusCode,
  lvl: Lvl,
}
impl Default for ErrResidual {
  fn default() -> Self {
    Self {
      err_msg: String::new(),
      pub_msg: "Internal server error".to_string(),
      field_errors: None,
      with_bt: true,
      context: None,
      status_code: StatusCode::INTERNAL_SERVER_ERROR,
      lvl: Lvl::ERROR,
    }
  }
}

impl ErrResidual {
  fn new(msg: impl ToString) -> Self {
    Self {
      err_msg: msg.to_string(),
      ..Default::default()
    }
  }
}

impl<T> Try for PossibleApiErr<T> {
  type Output = T;
  type Residual = ErrResidual;

  fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
    match self {
      PossibleApiErr::Ok(v) => ControlFlow::Continue(v),
      PossibleApiErr::Err(err) => ControlFlow::Break(err),
    }
  }
  fn from_output(output: Self::Output) -> Self {
    PossibleApiErr::Ok(output)
  }
}

impl<T> FromResidual for PossibleApiErr<T> {
  fn from_residual(residual: <Self as Try>::Residual) -> Self {
    PossibleApiErr::Err(residual)
  }
}
impl<T, E: From<ApiErr>> FromResidual<ErrResidual> for Result<T, E> {
  fn from_residual(r: ErrResidual) -> Self {
    Err(ApiErr::from(r).into())
  }
}

impl From<ErrResidual> for ApiErr {
  fn from(value: ErrResidual) -> Self {
    let mut event_msg = vec![];
    if let Some(context) = value.context {
      event_msg.push(context);
    }
    event_msg.push(value.err_msg.clone());
    if value.with_bt {
      let bt = Backtrace::new();
      let printer = bt_printer();
      event_msg.push(printer.format_trace_to_string(&bt).unwrap());
    }

    match value.lvl {
      Lvl::ERROR => event!(Level::ERROR, "{}", event_msg.join("\n")),
      Lvl::INFO => event!(Level::INFO, "{}", event_msg.join("\n")),
      Lvl::WARN => event!(Level::WARN, "{}", event_msg.join("\n")),
    }

    println!("??");

    Self {
      status_code: value.status_code,
      message: value.pub_msg,
      field_errors: value.field_errors.map(Into::into),
    }
  }
}

enum Lvl {
  ERROR,
  WARN,
  INFO,
}

// This function attempts to turn database errors into formatted constraint errors
// that the front-end can easily display. Right now it's just unique and null constraints.
//fn extract_fielderrors(err: DbErr) -> (DbErr, FieldErrors) {
//  let mut errors = FieldErrors::new();
//
//  let DbErr::Query(RuntimeErr::SqlxError(sqlx_err)) = &err else {
//    return (err, errors);
//  };
//  let Some(sqlx_err) = sqlx_err.as_database_error() else {
//    return (err, errors);
//  };
//  let pg_err = sqlx_err.downcast_ref() as &SqlxPostgresError;
//  match pg_err.code() {
//    // not_null_violation
//    "23502" => {
//      if let Some(column) = pg_err.column() {
//        errors.add_error(column, "must be present");
//      }
//    }
//    // unique_violation
//    "23505" => {
//      if let Some(detail) = pg_err.detail() {
//        if let Some(caps) = UNIQUE_CONSTRAINT_REGEX.captures(detail) {
//          let fields = caps["names"].split(", ");
//          for field in fields {
//            errors.add_error(field, "already exists");
//          }
//        }
//      }
//    }
//    _ => {}
//  }
//
//  (err, errors)
//}
