use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct User {
  pub id: String,
  pub email: String,
  pub phone: String,
  pub first_name: Option<String>,
  pub last_name: Option<String>,
  pub image: Option<String>,
  pub password: Option<String>,
  pub linked_in_profile: Option<String>,
  pub created_at: NaiveDateTime,
  pub updated_at: NaiveDateTime,
}

impl User {
  pub fn find_by_id(id: String) -> Result<Option<Self>> {
    Ok(None)
  }

  pub fn find_by_email(email: String) -> Result<Option<Self>> {
    Ok(None)
  }
}

#[derive(Serialize, specta::Type)]
pub struct FilteredUser {
  id: String,
  email: String,
  phone: String,
  first_name: Option<String>,
  last_name: Option<String>,
  image: Option<String>,
  created_at: NaiveDateTime,
  updated_at: NaiveDateTime,
}

impl From<&User> for FilteredUser {
  fn from(user: &User) -> Self {
    Self {
      id: user.id.clone(),
      email: user.email.clone(),
      phone: user.phone.clone(),
      first_name: user.first_name.clone(),
      last_name: user.last_name.clone(),
      image: user.image.clone(),
      created_at: user.created_at,
      updated_at: user.updated_at,
    }
  }
}
