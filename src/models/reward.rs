use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Reward {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub point_cost: i32,
    pub stock: i32,
    pub active: bool,
}
