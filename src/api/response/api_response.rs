use axum::response::IntoResponse;
use serde::Serialize;

#[derive(Serialize, Default, specta::Type)]
pub struct ApiResponse<T> {
  pub body: T,
}

impl<T> IntoResponse for ApiResponse<T>
where
  T: Serialize,
{
  fn into_response(self) -> axum::response::Response {
    axum::Json(self).into_response()
  }
}
