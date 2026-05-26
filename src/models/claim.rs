use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// JWT ID — utilisé pour les tokens QR à usage unique.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
}
