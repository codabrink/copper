use crate::prelude::*;
use axum::{
  handler::HandlerWithoutStateExt,
  http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderValue, Method, StatusCode,
  },
  middleware,
  routing::*,
};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};

pub mod response;

pub struct AppState {
  pool: Pool<Postgres>,
  config: Config,
}

impl AppState {
  async fn new() -> Result<Self> {
    let config = Config::init();
    let pool = PgPoolOptions::new()
      .max_connections(5)
      .connect(&config.database_url)
      .await?;

    Ok(Self { config, pool })
  }
}

pub async fn serve() -> Result<()> {
  let app_state = Arc::new(AppState::new().await?);
  let mut app = router(app_state.clone()).layer(TraceLayer::new_for_http());

  if let Some(cors_origin) = app_state.config.cors_origin.as_ref() {
    let cors = CorsLayer::new()
      .allow_origin(cors_origin.parse::<HeaderValue>().unwrap())
      .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
      .allow_credentials(true)
      .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    app = app.layer(cors);
  }

  let listener = tokio::net::TcpListener::bind(&app_state.config.host).await?;
  axum::serve(listener, app).await?;

  Ok(())
}

pub fn router(app_state: Arc<AppState>) -> Router {
  Router::new().nest("/", Router::new()).with_state(app_state)
}
