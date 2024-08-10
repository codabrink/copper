pub struct Config {
  pub database_url: String,
  pub host: String,
  pub cors_origin: Option<String>,
  pub jwt_secret: String,
}

impl Config {
  pub fn init() -> Self {
    Self {
      database_url: var("DATABASE_URL"),
      cors_origin: opt_var("CORS_ORIGIN"),
      host: var("HOST"),
      jwt_secret: var("JWT_SECRET"),
    }
  }
}

fn var(key: &str) -> String {
  std::env::var(key).expect(&format!("{key} must be set"))
}
fn opt_var(key: &str) -> Option<String> {
  std::env::var(key).ok()
}
