use axum::http::StatusCode;
use std::{
  collections::HashMap,
  ops::{ControlFlow, FromResidual},
};

use super::api_error::ApiErr;

#[derive(Debug)]
pub struct FieldErrors(HashMap<String, Vec<String>>);

impl FieldErrors {
  pub fn new() -> Self {
    Self(HashMap::new())
  }
  pub fn add_error(&mut self, field: &str, msg: &str) {
    let entry = self.0.entry(field.to_owned()).or_default();
    entry.push(msg.to_owned());
  }
}

impl From<FieldErrors> for HashMap<String, Vec<String>> {
  fn from(value: FieldErrors) -> Self {
    value.0
  }
}

impl From<FieldErrors> for ApiErr {
  fn from(value: FieldErrors) -> Self {
    ApiErr {
      field_errors: Some(value.0),
      status_code: StatusCode::BAD_REQUEST,
      message: "Unable to save record".to_string(),
    }
  }
}

impl std::ops::Try for FieldErrors {
  type Output = Self;
  type Residual = Self;
  fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
    match self.0.is_empty() {
      true => ControlFlow::Continue(self),
      false => ControlFlow::Break(self),
    }
  }
  fn from_output(output: Self::Output) -> Self {
    output
  }
}
impl FromResidual for FieldErrors {
  fn from_residual(residual: <Self as std::ops::Try>::Residual) -> Self {
    residual
  }
}
impl<T, E: From<ApiErr>> FromResidual<FieldErrors> for Result<T, E> {
  fn from_residual(r: FieldErrors) -> Self {
    Err(ApiErr::from(r).into())
  }
}
