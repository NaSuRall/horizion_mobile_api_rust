use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: i32,
    pub title: String,
    pub location: String,
    pub badge: String,
    pub badge_type: String,
    pub event_date: NaiveDate,
}
