use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
#[allow(dead_code)]
pub struct AppConfig {
    pub admin_hash: String
}
