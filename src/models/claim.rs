use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}
