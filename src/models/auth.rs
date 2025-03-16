use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Auth {
    /// Whether authentication is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Authentication realm (displayed in browser prompt)
    #[serde(default = "default_realm")]
    pub realm: String,
    /// Username for basic auth
    #[serde(default = "default_username")]
    pub username: String,
    /// Password for basic auth (will be hashed)
    #[serde(default = "default_password")]
    pub password: String,
}

fn default_realm() -> String { "Restricted Area".to_string() }
fn default_username() -> String { "x".to_string() }
fn default_password() -> String { "x".to_string() }

impl Default for Auth {
    fn default() -> Self {
        Self {
            enabled: false,
            realm: default_realm(),
            username: default_username(),
            password: default_password(),
        }
    }
}